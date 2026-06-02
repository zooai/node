// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// See the file LICENSE for licensing terms.

// Package vm provides the ZOO EVM + DEX VM factory and plugin runners.
//
// The zood binary bundles BOTH virtual machines:
//   - ZOO EVM: Lux EVM with 13+ precompiles (PQ crypto, threshold sigs, DEX, etc.)
//   - ZOO DEX: High-performance CLOB + AMM orderbook engine
//
// When launched as a plugin subprocess (LUX_VM_TRANSPORT set), it enters
// EVM plugin mode. The DEX VM is registered directly with the node's VM
// manager and runs in-process (no subprocess needed).
package vm

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/luxfi/evm/plugin/evm"
	"github.com/luxfi/evm/plugin/runner"
	"github.com/luxfi/ids"
	"github.com/luxfi/log"
	"github.com/luxfi/node/vms/dexvm"
	luxversion "github.com/luxfi/version"
	"github.com/luxfi/vm/rpc"

	// ZOO precompiles -- blank imports trigger init() registration.

	// AI Mining (0x0300) — Proof of AI: GPU/CPU compute rewards, model hosting, data sharing
	_ "github.com/luxfi/precompile/ai"

	// Post-quantum cryptography
	_ "github.com/luxfi/precompile/blake3"
	_ "github.com/luxfi/precompile/mldsa"
	_ "github.com/luxfi/precompile/mlkem"
	_ "github.com/luxfi/precompile/pqcrypto"
	_ "github.com/luxfi/precompile/slhdsa"

	// Threshold signatures
	_ "github.com/luxfi/precompile/cggmp21"
	_ "github.com/luxfi/precompile/corona"
	_ "github.com/luxfi/precompile/frost"

	// Curves & migration
	_ "github.com/luxfi/precompile/ed25519"
	_ "github.com/luxfi/precompile/secp256r1"
	_ "github.com/luxfi/precompile/sr25519"

	// DEX precompile (V4 pool manager at 0x9010)
	_ "github.com/luxfi/precompile/dex"

	// Encryption & privacy (FHE at 0x0700 — CKKS/TFHE encrypted compute)
	_ "github.com/luxfi/precompile/fhe"
	_ "github.com/luxfi/precompile/hpke"
	_ "github.com/luxfi/precompile/ring"

	// Zero-knowledge & graph
	_ "github.com/luxfi/precompile/graph"
	_ "github.com/luxfi/precompile/zk"
)

// VM IDs — all three are always available in the zood binary.
var (
	// EVMID is the Zoo EVM VM identifier.
	// Distinct from C-chain EVM ID — Zoo EVM runs only on Zoo chains.
	EVMID = ids.FromStringOrPanic("2n2njofjYvece8gZWCNnc1mqkcqfW6kbrhPRZPVzwxrSQrQ4gE")

	// DEXVMID is the DEX VM identifier: [32]byte{'d','e','x','v','m'}.
	DEXVMID = ids.ID(dexvm.VMID)

	// FHEVMID is the FHE VM identifier for encrypted computation.
	// Provides CKKS/TFHE operations: private inference, encrypted portfolio
	// management, confidential securities compliance checks.
	FHEVMID = ids.FromStringOrPanic("RqxGi72cio4QcProSVa5nmsd8weLrrKrvBieZRoPD4HxQYp8L")

	// DEXFactory creates new DEX VM instances.
	DEXFactory = &dexvm.Factory{}
)

// VMID returns the Zoo EVM VM ID string.
func VMID() string {
	return EVMID.String()
}

// StandardEVMID returns the standard C-chain EVM ID (from luxfi/evm).
// Use this when referencing the built-in C-chain VM, not the Zoo VM.
func StandardEVMID() ids.ID {
	return evm.ID
}

// IsDEXPlugin returns true if this subprocess was launched for the DEX VM.
// The node looks up plugins by VM ID filename in the plugin directory, so
// os.Args[0] will end with the DEX VM ID string when launched for the DEX VM.
func IsDEXPlugin() bool {
	exe := filepath.Base(os.Args[0])
	return strings.Contains(exe, DEXVMID.String())
}

// RunPlugin runs this binary as an EVM subprocess (plugin mode).
// Called when LUX_VM_TRANSPORT is set, indicating the node launched us
// as a child process for the EVM.
func RunPlugin() {
	versionStr := fmt.Sprintf("ZOO-EVM/%s [node=%s, rpcchainvm=%d]",
		evm.Version, luxversion.Current, luxversion.RPCChainVMProtocol)

	if len(os.Args) > 1 && os.Args[1] == "version" {
		fmt.Println(versionStr)
		os.Exit(0)
	}

	runner.Run(versionStr)
}

// RunDEXPlugin runs this binary as a DEX VM subprocess (plugin mode).
// Called when LUX_VM_TRANSPORT is set and the binary was invoked via the
// DEX VM plugin symlink.
func RunDEXPlugin() {
	versionStr := fmt.Sprintf("ZOO-DEX/1.0.0 [node=%s, rpcchainvm=%d]",
		luxversion.Current, luxversion.RPCChainVMProtocol)

	if len(os.Args) > 1 && os.Args[1] == "version" {
		fmt.Println(versionStr)
		os.Exit(0)
	}

	chainVM := dexvm.NewChainVM(log.Root())
	fmt.Printf("Starting %s\n", versionStr)
	if err := rpc.Serve(context.Background(), log.Root(), chainVM); err != nil {
		fmt.Printf("rpc.Serve error: %s\n", err)
		os.Exit(1)
	}
}
