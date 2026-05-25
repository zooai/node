// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// See the file LICENSE for licensing terms.

package main

import (
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/luxfi/constants"
	"github.com/luxfi/crypto/bls"
	"github.com/luxfi/crypto/hash"
	"github.com/luxfi/crypto/secp256k1"
	"github.com/luxfi/ids"
	"github.com/luxfi/log"
	"github.com/luxfi/node/config/node"
	"github.com/luxfi/node/genesis/builder"

	"github.com/luxfi/go-bip39"

	genesiscfg "github.com/luxfi/genesis/pkg/genesis"
)

const (
	devNetworkFile = "dev-network.json"

	// Must match upstream config.go constants.
	devHundredYears  = 100 * 365 * 24 * 60 * 60
	devOneMillionLUX = 1_000_000_000_000_000     // 1M LUX in nLUX
	devOneBillionLUX = 1_000_000_000_000_000_000 // 1B LUX in nLUX
)

// patchDevGenesis fixes the ETH->secp256k1 address mismatch in dev-mode genesis.
//
// The upstream builder.ForDevMode sets both ETHAddr and LUXAddr to the same
// value (RewardAddress). On P-Chain and X-Chain, addresses are derived as
// RIPEMD160(SHA256(compressed_secp256k1_pubkey)), not keccak256. When the
// genesis allocates to an ETH address, the P-Chain wallet cannot find UTXOs.
//
// This function loads a secp256k1 key from MNEMONIC/LIGHT_MNEMONIC or LUX_PRIVATE_KEY,
// derives both address formats, rebuilds genesis with correct addresses, and
// overwrites nodeConfig.GenesisBytes in place.
func patchDevGenesis(nodeConfig *node.Config, dataDir string) error {
	key, err := loadDevKey()
	if err != nil {
		log.Warn("dev mode: no wallet key available; P-Chain genesis UTXOs use ETH address format and will be invisible to wallet SDK",
			"hint", "set LUX_MNEMONIC, LIGHT_MNEMONIC, or LUX_PRIVATE_KEY to fix",
		)
		return nil // non-fatal
	}

	// secp256k1 address: RIPEMD160(SHA256(compressed_pubkey)) -- for P-Chain/X-Chain
	secpAddr := key.Address()

	// ETH address: keccak256(uncompressed_pubkey[1:])[12:] -- for C-Chain
	ethAddr := secp256k1.PubkeyToAddress(key.ToECDSA().PublicKey)
	var ethShortID ids.ShortID
	copy(ethShortID[:], ethAddr[:])

	log.Info("dev mode: deriving genesis addresses from wallet key",
		"pChainAddr", secpAddr,
		"cChainAddr", fmt.Sprintf("0x%x", ethAddr[:]),
	)

	// Derive NodeID from the staking TLS cert
	nodeID := ids.NodeIDFromCert(&ids.Certificate{
		Raw:       nodeConfig.StakingConfig.StakingTLSCert.Leaf.Raw,
		PublicKey: nodeConfig.StakingConfig.StakingTLSCert.Leaf.PublicKey,
	})

	// BLS credentials
	blsKey := nodeConfig.StakingConfig.StakingSigningKey
	blsPKBytes := bls.PublicKeyToCompressedBytes(blsKey.PublicKey())
	blsPK := fmt.Sprintf("0x%x", blsPKBytes)
	var blsPoP string
	if sig, err := blsKey.SignProofOfPossession(blsPKBytes); err == nil && sig != nil {
		blsPoP = fmt.Sprintf("0x%x", bls.SignatureToBytes(sig))
	}

	// Get start time: reuse from existing dev-network.json for determinism.
	// If no dev-network.json exists (first boot), use current time.
	startTime := readDevStartTime(dataDir)
	if startTime == 0 {
		startTime = uint64(time.Now().Unix())
	}

	// Build genesis config with separate ETH and secp256k1 addresses.
	// CRITICAL: The builder zeros InitialAmount for addresses in InitialStakedFunds.
	// So we use TWO allocations:
	//   1. Staking allocation (in InitialStakedFunds) — InitialAmount will be zeroed
	//   2. Spending allocation (same addr, NOT in InitialStakedFunds) — keeps its InitialAmount
	// The builder matches by address, so we use a derived "staker" address for staking
	// and the deployer address for spending.
	//
	// For the staker address, we use the node's own staking key (different key than deployer).
	// Use a deterministic but DIFFERENT address for staking (NOT the deployer).
	// This ensures the deployer's spending allocation is never in InitialStakedFunds.
	var stakerAddr ids.ShortID
	stakerHash := hash.ComputeHash160([]byte("zoo-dev-staker-v1"))
	copy(stakerAddr[:], stakerHash)
	cfg := &genesiscfg.Config{
		NetworkID: constants.CustomID,
		Allocations: []genesiscfg.Allocation{
			// Staking allocation — builder will zero InitialAmount since it's in InitialStakedFunds
			{
				EVMAddr:       stakerAddr, // doesn't matter for P-chain, just needs to be valid
				UTXOAddr:       stakerAddr,
				InitialAmount: 0,
				UnlockSchedule: []genesiscfg.LockedAmount{
					{Amount: devOneBillionLUX, Locktime: 0},
				},
			},
			// Spending allocation — NOT in InitialStakedFunds, so InitialAmount survives
			{
				EVMAddr:       ethShortID,
				UTXOAddr:       secpAddr,
				InitialAmount: devOneMillionLUX,
				UnlockSchedule: []genesiscfg.LockedAmount{},
			},
		},
		StartTime:                  startTime,
		InitialStakeDuration:       devHundredYears,
		InitialStakeDurationOffset: 0,
		InitialStakedFunds:         []ids.ShortID{stakerAddr}, // Only staker, NOT deployer
		InitialStakers: []genesiscfg.Staker{
			{
				NodeID:        nodeID,
				RewardAddress: secpAddr,
				DelegationFee: 1000000,
				Weight:        devOneBillionLUX,
				StartTime:     startTime,
				EndTime:       startTime + devHundredYears,
			},
		},
		CChainGenesis: devCChainGenesis(ethAddr),
		Message:       "Lux Development Mode Genesis",
	}

	if blsPK != "" && blsPoP != "" {
		cfg.InitialStakers[0].Signer = &genesiscfg.ProofOfPossession{
			PublicKey:         blsPK,
			ProofOfPossession: blsPoP,
		}
	}

	genesisBytes, luxAssetID, err := builder.FromConfig(cfg)
	if err != nil {
		return fmt.Errorf("failed to build patched dev genesis: %w", err)
	}

	// Skip if genesis already matches (correct dev-network.json from previous boot)
	oldHash := hash.ComputeHash256(nodeConfig.GenesisBytes)
	newHash := hash.ComputeHash256(genesisBytes)
	if string(oldHash) == string(newHash) {
		log.Info("dev mode: genesis already has correct secp256k1 addresses")
		return nil
	}

	// Genesis changed -- override in memory
	nodeConfig.GenesisBytes = genesisBytes
	nodeConfig.XAssetID = luxAssetID

	genesisHashID, _ := ids.ToID(newHash)

	log.Info("dev mode: patched genesis with correct secp256k1 addresses",
		"genesisHash", genesisHashID,
		"luxAssetID", luxAssetID,
	)

	// Persist corrected dev-network.json
	writeDevNetworkJSON(dataDir, genesisBytes, genesisHashID, luxAssetID, startTime, nodeID.String())

	// Remove stale DB if it exists (genesis hash changed, DB would reject it)
	dbPath := nodeConfig.DatabaseConfig.Path
	if info, err := os.Stat(dbPath); err == nil && info.IsDir() {
		log.Warn("dev mode: removing stale database (genesis hash changed)", "path", dbPath)
		if err := os.RemoveAll(dbPath); err != nil {
			return fmt.Errorf("failed to remove stale database at %s: %w", dbPath, err)
		}
	}

	return nil
}

// loadDevKey loads a secp256k1 private key from MNEMONIC/LIGHT_MNEMONIC or LUX_PRIVATE_KEY.
func loadDevKey() (*secp256k1.PrivateKey, error) {
	mnemonic := os.Getenv("MNEMONIC")
	if mnemonic == "" {
		mnemonic = os.Getenv("LIGHT_MNEMONIC")
	}
	if mnemonic != "" {
		mnemonic = strings.TrimSpace(mnemonic)
		if !bip39.IsMnemonicValid(mnemonic) {
			return nil, fmt.Errorf("invalid BIP39 mnemonic in MNEMONIC/LIGHT_MNEMONIC")
		}
		seed := bip39.NewSeed(mnemonic, "")
		if len(seed) < 32 {
			return nil, fmt.Errorf("seed too short")
		}
		return secp256k1.ToPrivateKey(seed[:32])
	}

	if keyHex := os.Getenv("PRIVATE_KEY"); keyHex != "" {
		keyHex = strings.TrimSpace(keyHex)
		keyHex = strings.TrimPrefix(keyHex, "0x")
		keyHex = strings.TrimPrefix(keyHex, "0X")
		keyBytes, err := hex.DecodeString(keyHex)
		if err != nil {
			return nil, fmt.Errorf("invalid hex in PRIVATE_KEY: %w", err)
		}
		return secp256k1.ToPrivateKey(keyBytes)
	}

	return nil, fmt.Errorf("neither MNEMONIC/LIGHT_MNEMONIC nor LUX_PRIVATE_KEY is set")
}

// readDevStartTime extracts the start time from an existing dev-network.json.
// Returns 0 if the file does not exist (first boot).
func readDevStartTime(dataDir string) uint64 {
	data, err := os.ReadFile(filepath.Join(dataDir, devNetworkFile))
	if err != nil {
		return 0
	}
	var cfg struct {
		StartTime uint64 `json:"startTime"`
	}
	if json.Unmarshal(data, &cfg) != nil {
		return 0
	}
	return cfg.StartTime
}

// writeDevNetworkJSON persists the corrected genesis to dev-network.json.
func writeDevNetworkJSON(dataDir string, genesisBytes []byte, genesisHash ids.ID, luxAssetID ids.ID, startTime uint64, nodeID string) {
	cfg := struct {
		Version      int    `json:"version"`
		StartTime    uint64 `json:"startTime"`
		NodeID       string `json:"nodeId"`
		GenesisBytes []byte `json:"genesisBytes"`
		GenesisHash  string `json:"genesisHash"`
		XAssetID     string `json:"xAssetId"`
	}{
		Version:      1,
		StartTime:    startTime,
		NodeID:       nodeID,
		GenesisBytes: genesisBytes,
		GenesisHash:  genesisHash.String(),
		XAssetID:     luxAssetID.String(),
	}

	data, err := json.MarshalIndent(cfg, "", "  ")
	if err != nil {
		log.Warn("dev mode: failed to marshal dev-network.json", "error", err)
		return
	}

	path := filepath.Join(dataDir, devNetworkFile)
	if err := os.WriteFile(path, data, 0644); err != nil {
		log.Warn("dev mode: failed to write dev-network.json", "error", err, "path", path)
	}
}

// devCChainGenesis generates a C-Chain genesis JSON that funds the wallet's
// ETH address plus the standard Anvil/Hardhat test accounts.
func devCChainGenesis(ethAddr [20]byte) string {
	return fmt.Sprintf(`{
  "config": {
    "chainId": 31337,
    "homesteadBlock": 0,
    "eip150Block": 0,
    "eip155Block": 0,
    "eip158Block": 0,
    "byzantiumBlock": 0,
    "constantinopleBlock": 0,
    "petersburgBlock": 0,
    "istanbulBlock": 0,
    "muirGlacierBlock": 0,
    "berlinBlock": 0,
    "londonBlock": 0,
    "arrowGlacierBlock": 0,
    "grayGlacierBlock": 0,
    "mergeNetsplitBlock": 0,
    "shanghaiTime": 0,
    "cancunTime": 0,
    "blobSchedule": {
      "cancun": {"target": 3, "max": 6, "baseFeeUpdateFraction": 3338477}
    },
    "terminalTotalDifficulty": 0,
    "chainEVMTimestamp": 0,
    "durangoTimestamp": 0,
    "etnaTimestamp": 253399622400,
    "feeConfig": {
      "gasLimit": 30000000,
      "targetBlockRate": 1,
      "minBaseFee": 1000000000,
      "targetGas": 100000000,
      "baseFeeChangeDenominator": 48,
      "minBlockGasCost": 0,
      "maxBlockGasCost": 1000000,
      "blockGasCostStep": 200000
    },
    "warpConfig": {
      "blockTimestamp": 0,
      "quorumNumerator": 67,
      "requirePrimaryNetworkSigners": false
    },
    "dexConfig": {
      "blockTimestamp": 0,
      "maxPools": 10000,
      "enableFlashLoans": true,
      "enableHooks": true
    }
  },
  "alloc": {
    "%x": {
      "balance": "0x200000000000000000000000000000000000000000000000000000000000000"
    },
    "f39Fd6e51aad88F6F4ce6aB8827279cffFb92266": {
      "balance": "0x200000000000000000000000000000000000000000000000000000000000000"
    },
    "70997970C51812dc3A010C7d01b50e0d17dc79C8": {
      "balance": "0x200000000000000000000000000000000000000000000000000000000000000"
    },
    "3C44CdDdB6a900fa2b585dd299e03d12FA4293BC": {
      "balance": "0x200000000000000000000000000000000000000000000000000000000000000"
    },
    "90F79bf6EB2c4f870365E785982E1f101E93b906": {
      "balance": "0x200000000000000000000000000000000000000000000000000000000000000"
    }
  },
  "nonce": "0x0",
  "timestamp": "0x0",
  "extraData": "0x",
  "gasLimit": "0x1c9c380",
  "difficulty": "0x0",
  "mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
  "coinbase": "0x0000000000000000000000000000000000000000",
  "number": "0x0",
  "gasUsed": "0x0",
  "parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
}`, ethAddr)
}

// nodeStakingAddr derives the secp256k1 address from the node's staking key.
func nodeStakingAddr(nc *node.Config) ids.ShortID {
	keyPath := nc.StakingKeyPath
	if keyPath == "" {
		return ids.ShortEmpty
	}
	keyBytes, err := os.ReadFile(keyPath)
	if err != nil {
		return ids.ShortEmpty
	}
	privKey, err := secp256k1.ToPrivateKey(keyBytes)
	if err != nil {
		return ids.ShortEmpty
	}
	return privKey.Address()
}
