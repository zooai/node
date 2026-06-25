// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// See the file LICENSE for licensing terms.

//go:build bootstrap
// +build bootstrap

package main

import (
	"context"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/luxfi/constants"
	"github.com/luxfi/crypto/secp256k1"
	"github.com/luxfi/ids"
	"github.com/luxfi/math/set"
	ptxs "github.com/luxfi/proto/p/txs"
	"github.com/luxfi/sdk/info"
	"github.com/luxfi/sdk/platformvm"
	"github.com/luxfi/sdk/wallet/primary"
	lux "github.com/luxfi/utxo"
	"github.com/luxfi/utxo/secp256k1fx"

	"github.com/luxfi/go-bip32"
	"github.com/luxfi/go-bip39"
)

// Well-known VM IDs.
var (
	// EVM VM ID — Zoo EVM (unique ID, does NOT conflict with C-chain).
	evmVMID = ids.FromStringOrPanic("2n2njofjYvece8gZWCNnc1mqkcqfW6kbrhPRZPVzwxrSQrQ4gE")

	// DEX VM ID — high-performance orderbook engine.
	dexVMID = ids.FromStringOrPanic("mDVT5EWMumBp3LCqvKwuyZQeY1VXr1jvjGNAt8nL4UFiXvqXr")
)

// chainSpec describes a chain to create within a subnet.
type chainSpec struct {
	Name        string
	VMID        ids.ID
	GenesisFile string
	Alias       string
}

// bootstrapConfig holds all configuration for an L1 bootstrap.
type bootstrapConfig struct {
	// Lux Network RPC to bootstrap from.
	NodeURI string

	// L1 identity.
	NetworkName string

	// Chains to create (EVM, DEX, or any combination).
	Chains []chainSpec

	// Key material — mnemonic or raw private key hex.
	Mnemonic   string
	PrivateKey string

	// BIP44 derivation path coin type (9000 for P/X-Chain, 60 for C-Chain/EVM).
	CoinType uint32

	// BIP44 key index (default 1 — index 0 is staker, has no unlocked balance).
	KeyIndex uint32

	// Validator weight for subnet validators.
	ValidatorWeight uint64

	// How long validators serve (from now).
	ValidatorDuration time.Duration

	// Existing subnet ID to reuse (skip subnet creation).
	SubnetID string
}

func runBootstrap(args []string) {
	cfg := parseBootstrapFlags(args)

	fmt.Println("╦    ╦  ╔═╗  ╦ ╦  ╦  ╦═╗  ╦  ╔╦╗  ╦ ╦")
	fmt.Println("║    ║  ║ ║  ║ ║  ║  ║ ║  ║   ║   ╚╦╝")
	fmt.Println("╩═╝  ╩  ╚═╩  ╚═╝  ╩  ╩═╝  ╩   ╩    ╩ ")
	fmt.Println()
	fmt.Printf("Bootstrapping L1 '%s' on Lux Network\n", cfg.NetworkName)
	fmt.Printf("  Bootnode:  %s\n", cfg.NodeURI)
	fmt.Printf("  Chains:    %d\n", len(cfg.Chains))
	for _, c := range cfg.Chains {
		fmt.Printf("    - %s (VM: %s)\n", c.Name, c.VMID)
	}
	fmt.Println()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Minute)
	defer cancel()

	// Derive private key.
	privKey := deriveKey(cfg)
	addr := privKey.Address()
	log.Printf("Key address: %s", addr)

	// Connect to Lux Network.
	infoClient := info.NewClient(cfg.NodeURI)
	nodeID, _, err := infoClient.GetNodeID(ctx)
	if err != nil {
		log.Fatalf("Cannot reach Lux bootnode at %s: %v", cfg.NodeURI, err)
	}
	log.Printf("Connected to bootnode: %s", nodeID)

	networkID, err := infoClient.GetNetworkID(ctx)
	if err != nil {
		log.Fatalf("Cannot get network ID: %v", err)
	}
	log.Printf("Lux Network ID: %d", networkID)

	// Fetch current validators.
	pClient := platformvm.NewClient(cfg.NodeURI)
	validators, err := pClient.GetCurrentValidators(ctx, ids.Empty, nil)
	if err != nil {
		log.Fatalf("Cannot get validators: %v", err)
	}
	log.Printf("Found %d validators on primary network", len(validators))

	var minEnd uint64
	for _, v := range validators {
		if minEnd == 0 || v.EndTime < minEnd {
			minEnd = v.EndTime
		}
	}

	// Build wallet.
	kc := secp256k1fx.NewKeychain(privKey)
	log.Printf("Keychain addresses: %v", kc.Addrs)
	log.Printf("Keychain keys: %d", len(kc.Keys))
	log.Printf("Key pubkey (compressed): %x", privKey.PublicKey().CompressedBytes())
	log.Printf("Key address (SHA256+RIPEMD160): %s", privKey.Address())
	adapter := primary.NewKeychainAdapter(kc)

	wallet, err := primary.MakeWallet(ctx, &primary.WalletConfig{
		URI: cfg.NodeURI, LUXKeychain: adapter, EVMKeychain: adapter,
	})
	if err != nil {
		log.Fatalf("Wallet creation failed: %v", err)
	}

	// Check balances across all chains.
	luxAssetID := wallet.X().Builder().Context().UTXOAssetID

	xBalance, err := wallet.X().Builder().GetFTBalance()
	if err != nil {
		log.Printf("X-Chain balance check failed: %v", err)
	} else {
		xBal := xBalance[luxAssetID]
		log.Printf("X-Chain balance: %d μLUX (%.4f LUX)", xBal, float64(xBal)/1e6)
	}

	pBalance, err := wallet.P().Builder().GetBalance()
	if err != nil {
		log.Fatalf("P-Chain balance check failed: %v", err)
	}
	pBal := pBalance[luxAssetID]
	log.Printf("P-Chain balance: %d μLUX (%.4f LUX)", pBal, float64(pBal)/1e6)

	// Fund P-Chain from X-Chain if needed (genesis allocates to X-Chain).
	// Skip if P-Chain balance is sufficient OR if X-Chain balance is also 0
	// (genesis may have time-locked P-Chain UTXOs visible via RPC but not via SDK).
	if pBal < 1_000_000_000 {
		xBal := uint64(0)
		if xBalance != nil {
			xBal = xBalance[luxAssetID]
		}
		if xBal > 0 {
			wallet = fundPChainFromX(ctx, cfg, wallet, addr, luxAssetID)
		} else {
			log.Printf("Both P-Chain and X-Chain SDK balances are 0 — trying subnet creation anyway (RPC may see unlocked UTXOs)")
		}
	}

	// Step 1: Create or reuse subnet.
	var subnetID ids.ID
	if cfg.SubnetID != "" {
		subnetID, err = ids.FromString(cfg.SubnetID)
		if err != nil {
			log.Fatalf("Invalid --subnet-id: %v", err)
		}
		log.Printf("Reusing existing subnet: %s", subnetID)
	} else {
		log.Printf("Creating subnet for %s...", cfg.NetworkName)
		owner := &secp256k1fx.OutputOwners{
			Threshold: 1,
			Addrs:     []ids.ShortID{addr},
		}

		utxos, utxoErr := wallet.P().Builder().GetBalance()
		if utxoErr != nil {
			log.Printf("WARNING: Builder.GetBalance failed: %v", utxoErr)
		} else {
			for asset, bal := range utxos {
				log.Printf("  Builder balance: asset=%s amount=%d", asset, bal)
			}
		}

		subnetTx, subnetErr := wallet.P().IssueCreateNetworkTx(owner)
		if subnetErr != nil {
			log.Fatalf("Subnet creation failed: %v", subnetErr)
		}
		subnetID = subnetTx.ID()
		log.Printf("Subnet ID: %s", subnetID)
		waitForAcceptance("subnet")
	}

	// Step 2: Create chains.
	type chainResult struct {
		Name         string
		BlockchainID ids.ID
		VMID         ids.ID
		Alias        string
	}
	var results []chainResult

	// Build a set of tx IDs the wallet must fetch (subnet creation tx).
	txsToFetch := set.Of(subnetID)

	for i, chain := range cfg.Chains {
		genesisBytes, err := os.ReadFile(chain.GenesisFile)
		if err != nil {
			log.Fatalf("Cannot read genesis for %s: %v", chain.Name, err)
		}
		log.Printf("Creating chain '%s' (%d bytes genesis, VM %s)...", chain.Name, len(genesisBytes), chain.VMID)

		// Re-sync wallet — include the subnet tx so the owner cache is populated.
		w, err := primary.MakeWallet(ctx, &primary.WalletConfig{
			URI: cfg.NodeURI, LUXKeychain: adapter, EVMKeychain: adapter,
			PChainTxsToFetch: txsToFetch,
		})
		if err != nil {
			log.Fatalf("Wallet re-sync for chain %d failed: %v", i, err)
		}

		chainTx, err := w.P().IssueCreateChainTx(subnetID, genesisBytes, chain.VMID, nil, chain.Name)
		if err != nil {
			log.Printf("WARNING: Chain creation for '%s' failed: %v (skipping)", chain.Name, err)
			continue
		}

		results = append(results, chainResult{
			Name:         chain.Name,
			BlockchainID: chainTx.ID(),
			VMID:         chain.VMID,
			Alias:        chain.Alias,
		})
		log.Printf("Chain '%s' blockchain ID: %s", chain.Name, chainTx.ID())
		waitForAcceptance(chain.Name)
	}

	// Step 3: Add validators to subnet.
	if len(validators) > 0 {
		log.Printf("Adding %d validators to subnet %s...", len(validators), subnetID)
		w, err := primary.MakeWallet(ctx, &primary.WalletConfig{
			URI: cfg.NodeURI, LUXKeychain: adapter, EVMKeychain: adapter,
			PChainTxsToFetch: txsToFetch,
		})
		if err != nil {
			log.Printf("WARNING: validator wallet sync failed: %v", err)
		} else {
			startTime := time.Now().Add(60 * time.Second)
			endTime := startTime.Add(cfg.ValidatorDuration)
			if minEnd > 0 {
				primaryEnd := time.Unix(int64(minEnd), 0).Add(-1 * time.Hour)
				if primaryEnd.Before(endTime) {
					endTime = primaryEnd
				}
			}

			for _, v := range validators {
				_, err := w.P().IssueAddChainValidatorTx(&ptxs.ChainValidator{
					Validator: ptxs.Validator{
						NodeID: v.NodeID,
						Start:  uint64(startTime.Unix()),
						End:    uint64(endTime.Unix()),
						Wght:   cfg.ValidatorWeight,
					},
					Chain: subnetID,
				})
				if err != nil {
					log.Printf("WARNING: add validator %s: %v", v.NodeID, err)
				} else {
					log.Printf("Validator added: %s", v.NodeID)
				}
				time.Sleep(1 * time.Second)
			}
		}
	}

	// Output results.
	fmt.Println()
	fmt.Println("════════════════════════════════════════════════════════")
	fmt.Printf("  %s L1 Bootstrap Complete\n", cfg.NetworkName)
	fmt.Println("════════════════════════════════════════════════════════")
	fmt.Printf("  Subnet ID: %s\n", subnetID)
	fmt.Println()

	for _, r := range results {
		fmt.Printf("  Chain: %s\n", r.Name)
		fmt.Printf("    Blockchain ID: %s\n", r.BlockchainID)
		fmt.Printf("    VM ID:         %s\n", r.VMID)
		fmt.Printf("    Alias:         %s\n", r.Alias)
		fmt.Printf("    RPC:           %s/rpc  (internal: /ext/bc/%s/rpc)\n", cfg.NodeURI, r.BlockchainID)
		fmt.Printf("    WS:            %s/ws   (internal: /ext/bc/%s/ws)\n", cfg.NodeURI, r.BlockchainID)
		fmt.Printf("    ZAP:           %s/zap  (internal: /ext/bc/%s/zap)\n", cfg.NodeURI, r.BlockchainID)
		fmt.Println()
	}

	fmt.Println("  Validators:", len(validators))
	fmt.Println()

	// Output K8s CRD YAML for the operator.
	fmt.Println("  LuxNetwork CRD chains entry:")
	fmt.Println("  chains:")
	for _, r := range results {
		fmt.Printf("    - alias: %s\n", r.Alias)
		fmt.Printf("      blockchainId: %s\n", r.BlockchainID)
		fmt.Printf("      trackingId: %s\n", subnetID)
	}
	fmt.Println()

	// Output JSON for automation.
	output := map[string]interface{}{
		"network":    cfg.NetworkName,
		"subnetId":   subnetID.String(),
		"validators": len(validators),
		"chains":     []map[string]string{},
	}
	for _, r := range results {
		output["chains"] = append(output["chains"].([]map[string]string), map[string]string{
			"name":         r.Name,
			"blockchainId": r.BlockchainID.String(),
			"vmId":         r.VMID.String(),
			"alias":        r.Alias,
		})
	}
	jsonBytes, _ := json.MarshalIndent(output, "", "  ")
	_ = os.WriteFile("bootstrap-result.json", jsonBytes, 0644)
	fmt.Println("  Result written to bootstrap-result.json")
	fmt.Println("════════════════════════════════════════════════════════")
}

func parseBootstrapFlags(args []string) bootstrapConfig {
	cfg := bootstrapConfig{
		NodeURI:           envOr("LUX_URI", "https://api.lux-dev.network"),
		NetworkName:       envOr("NETWORK_NAME", "Zoo"),
		Mnemonic:          mnemonicFromEnv(),
		PrivateKey:        os.Getenv("LUX_PRIVATE_KEY"),
		// P/X-Chain genesis allocations are derived at the canonical Lux BIP44
		// path m/44'/9000'/0'/0/{i} (see luxfi/genesis keys.go LoadKeysFromMnemonic).
		// Subnet/chain creation spends P-Chain UTXOs, so the funding key must use
		// coin type 9000, not 60 (60 is the C-Chain/EVM path). Override with COIN_TYPE.
		CoinType:          uint32(envIntOr("COIN_TYPE", 9000)),
		KeyIndex:          uint32(envIntOr("KEY_INDEX", 0)), // index 0 has genesis allocation
		ValidatorWeight:   20,
		ValidatorDuration: 300 * 24 * time.Hour, // ~10 months
	}

	// Default chains: Zoo EVM + Zoo DEX.
	evmGenesis := envOr("EVM_GENESIS", "./genesis.json")
	dexGenesis := envOr("DEX_GENESIS", "./cmd/deploy-dex/genesis.json")
	chainsEnv := envOr("CHAINS", "evm,dex")

	chains := strings.Split(chainsEnv, ",")
	for _, c := range chains {
		switch strings.TrimSpace(strings.ToLower(c)) {
		case "evm":
			cfg.Chains = append(cfg.Chains, chainSpec{
				Name:        envOr("EVM_CHAIN_NAME", ""),
				VMID:        evmVMID,
				GenesisFile: evmGenesis,
				Alias:       "zooevm",
			})
		case "dex":
			cfg.Chains = append(cfg.Chains, chainSpec{
				Name:        envOr("DEX_CHAIN_NAME", ""),
				VMID:        dexVMID,
				GenesisFile: dexGenesis,
				Alias:       "zoodex",
			})
		default:
			log.Fatalf("Unknown chain type: %s (supported: evm, dex)", c)
		}
	}

	// Parse any additional flags from args.
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--uri":
			if i+1 < len(args) {
				cfg.NodeURI = args[i+1]
				i++
			}
		case "--name":
			if i+1 < len(args) {
				cfg.NetworkName = args[i+1]
				i++
			}
		case "--subnet-id":
			if i+1 < len(args) {
				cfg.SubnetID = args[i+1]
				i++
			}
		case "--chains":
			if i+1 < len(args) {
				// Re-parse chains from flag.
				cfg.Chains = nil
				for _, c := range strings.Split(args[i+1], ",") {
					switch strings.TrimSpace(strings.ToLower(c)) {
					case "evm":
						cfg.Chains = append(cfg.Chains, chainSpec{
							Name:        envOr("EVM_CHAIN_NAME", ""),
							VMID:        evmVMID,
							GenesisFile: evmGenesis,
							Alias:       "zooevm",
						})
					case "dex":
						cfg.Chains = append(cfg.Chains, chainSpec{
							Name:        envOr("DEX_CHAIN_NAME", ""),
							VMID:        dexVMID,
							GenesisFile: dexGenesis,
							Alias:       "zoodex",
						})
					}
				}
				i++
			}
		}
	}

	if cfg.Mnemonic == "" && cfg.PrivateKey == "" {
		log.Fatal("Set LUX_MNEMONIC, LIGHT_MNEMONIC, or LUX_PRIVATE_KEY to fund subnet creation")
	}

	return cfg
}

func deriveKey(cfg bootstrapConfig) *secp256k1.PrivateKey {
	if cfg.PrivateKey != "" {
		keyBytes, err := hex.DecodeString(cfg.PrivateKey)
		if err != nil {
			log.Fatalf("Invalid LUX_PRIVATE_KEY hex: %v", err)
		}
		// Copy before creating PrivateKey (ToPrivateKey stores a reference, not a copy).
		keyCopy := make([]byte, len(keyBytes))
		copy(keyCopy, keyBytes)
		privKey, err := secp256k1.ToPrivateKey(keyCopy)
		if err != nil {
			log.Fatalf("Invalid private key: %v", err)
		}
		// Zero the raw hex-decoded bytes (safe — privKey has its own copy).
		for i := range keyBytes {
			keyBytes[i] = 0
		}
		return privKey
	}

	// BIP44 derivation: m/44'/{coinType}'/0'/0/{keyIndex}
	seed := bip39.NewSeed(cfg.Mnemonic, "")
	defer func() {
		for i := range seed {
			seed[i] = 0
		}
	}()

	masterKey, err := bip32.NewMasterKey(seed)
	if err != nil {
		log.Fatalf("BIP32 master key: %v", err)
	}
	defer func() {
		if masterKey != nil {
			for i := range masterKey.Key {
				masterKey.Key[i] = 0
			}
		}
	}()

	k, err := masterKey.NewChildKey(bip32.FirstHardenedChild + 44)
	if err != nil {
		log.Fatalf("BIP32 derivation at purpose: %v", err)
	}
	k, err = k.NewChildKey(bip32.FirstHardenedChild + cfg.CoinType)
	if err != nil {
		log.Fatalf("BIP32 derivation at coin type: %v", err)
	}
	k, err = k.NewChildKey(bip32.FirstHardenedChild + 0)
	if err != nil {
		log.Fatalf("BIP32 derivation at account: %v", err)
	}
	k, err = k.NewChildKey(0)
	if err != nil {
		log.Fatalf("BIP32 derivation at change: %v", err)
	}
	k, err = k.NewChildKey(cfg.KeyIndex)
	if err != nil {
		log.Fatalf("BIP32 derivation at index: %v", err)
	}

	// BIP32 keys must be exactly 32 bytes for secp256k1. The go-bip32 library
	// may return fewer bytes if the key has leading zeros (big.Int strips them).
	// Left-pad with zeros to normalize to 32 bytes (big-endian).
	var keyBytes [32]byte
	if len(k.Key) > 32 {
		log.Fatalf("BIP32 key too long: %d bytes", len(k.Key))
	}
	copy(keyBytes[32-len(k.Key):], k.Key)

	privKey, err := secp256k1.ToPrivateKey(keyBytes[:])
	if err != nil {
		log.Fatalf("Derived key invalid: %v", err)
	}

	// Zero intermediate key material (safe now — privKey has its own copy).
	for i := range k.Key {
		k.Key[i] = 0
	}

	return privKey
}

func fundPChainFromX(ctx context.Context, cfg bootstrapConfig, wallet primary.Wallet, addr ids.ShortID, assetID ids.ID) primary.Wallet {
	log.Println("Exporting LUX from X-Chain → P-Chain...")
	exportAmount := uint64(10_000_000_000) // 10 LUX

	_, err := wallet.X().IssueExportTx(
		constants.PlatformChainID,
		[]*lux.TransferableOutput{{
			Asset: lux.Asset{ID: assetID},
			Out: &secp256k1fx.TransferOutput{
				Amt: exportAmount,
				OutputOwners: secp256k1fx.OutputOwners{
					Threshold: 1,
					Addrs:     []ids.ShortID{addr},
				},
			},
		}},
	)
	if err != nil {
		log.Fatalf("X→P export failed (ensure key has X-Chain LUX balance): %v", err)
	}
	log.Println("X→P export submitted, importing on P-Chain...")

	_, err = wallet.P().IssueImportTx(
		wallet.X().Builder().Context().BlockchainID,
		&secp256k1fx.OutputOwners{
			Threshold: 1,
			Addrs:     []ids.ShortID{addr},
		},
	)
	if err != nil {
		log.Fatalf("P-Chain import failed: %v", err)
	}
	log.Println("P-Chain funded successfully via X→P export/import")
	return wallet
}

func fundPChain(ctx context.Context, cfg bootstrapConfig, wallet primary.Wallet, addr ids.ShortID, privKey *secp256k1.PrivateKey) primary.Wallet {
	log.Println("P-Chain balance insufficient. Checking X-Chain and C-Chain...")

	// Check X-Chain balance first (genesis allocates to X-Chain)
	xBal := wallet.X().Builder().Context().UTXOAssetID
	log.Printf("X-Chain asset ID: %s", xBal)

	if wallet.C() == nil {
		log.Fatal("C-Chain wallet not available — cannot fund P-Chain. Genesis should allocate directly to P-Chain or use X→P export.")
	}

	exportAmount := uint64(10_000_000_000) // 10 LUX
	cExportTx, err := wallet.C().IssueExportTx(
		constants.PlatformChainID,
		[]*secp256k1fx.TransferOutput{{
			Amt: exportAmount,
			OutputOwners: secp256k1fx.OutputOwners{
				Threshold: 1,
				Addrs:     []ids.ShortID{addr},
			},
		}},
	)
	if err != nil {
		log.Fatalf("C→P export failed (ensure key has C-Chain LUX balance): %v", err)
	}
	_ = cExportTx

	waitForAcceptance("C→P export")

	cChainID := wallet.C().Builder().Context().BlockchainID
	_, err = wallet.P().IssueImportTx(cChainID, &secp256k1fx.OutputOwners{
		Threshold: 1, Addrs: []ids.ShortID{addr},
	})
	if err != nil {
		log.Fatalf("P-Chain import failed: %v", err)
	}

	waitForAcceptance("P-Chain import")

	kc := secp256k1fx.NewKeychain(privKey)
	adapter := primary.NewKeychainAdapter(kc)
	w, err := primary.MakeWallet(ctx, &primary.WalletConfig{
		URI: cfg.NodeURI, LUXKeychain: adapter, EVMKeychain: adapter,
	})
	if err != nil {
		log.Fatalf("Wallet re-sync failed: %v", err)
	}
	return w
}

func waitForAcceptance(label string) {
	log.Printf("Waiting 10s for %s tx acceptance...", label)
	time.Sleep(10 * time.Second)
}

func envOr(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

// envIntOr returns the integer value of an env var, or fallback if unset/invalid.
func envIntOr(key string, fallback int) int {
	if v := os.Getenv(key); v != "" {
		if n, err := strconv.Atoi(v); err == nil {
			return n
		}
	}
	return fallback
}

// mnemonicFromEnv returns mnemonic from LUX_MNEMONIC or LIGHT_MNEMONIC.
func mnemonicFromEnv() string {
	if m := os.Getenv("LUX_MNEMONIC"); m != "" {
		return m
	}
	return os.Getenv("LIGHT_MNEMONIC")
}
