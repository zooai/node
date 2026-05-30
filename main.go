// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// See the file LICENSE for licensing terms.

// zood is the Zoo Network node — a sovereign L1 on the Lux Network.
//
// Usage:
//
//	zood                    Run the node (default)
//	zood bootstrap          Bootstrap a new L1 (EVM + DEX) on Lux Network
//	zood version            Print version info
//
// When invoked as a VM subprocess (LUX_VM_TRANSPORT set), it enters EVM
// plugin mode — no subcommand needed. This lets a single binary serve as
// both the node process and the EVM plugin.
package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"os"
	"path/filepath"
	"runtime/debug"
	"strconv"

	"github.com/spf13/pflag"
	"golang.org/x/term"

	"github.com/luxfi/node/app"
	"github.com/luxfi/node/config"
	nodeversion "github.com/luxfi/node/version"

	"github.com/zooai/node/vm"
)

const header = `
╦    ╦  ╔═╗  ╦ ╦  ╦  ╦═╗  ╦  ╔╦╗  ╦ ╦
║    ║  ║ ║  ║ ║  ║  ║ ║  ║   ║   ╚╦╝
╩═╝  ╩  ╚═╩  ╚═╝  ╩  ╩═╝  ╩   ╩    ╩
`

func main() {
	// VM subprocess mode — the node launched us as a plugin.
	// Detect which VM based on the executable name (symlink target).
	if os.Getenv("LUX_VM_TRANSPORT") != "" {
		if vm.IsDEXPlugin() {
			vm.RunDEXPlugin()
		} else {
			vm.RunPlugin()
		}
		return
	}

	// Subcommand dispatch.
	if len(os.Args) > 1 {
		switch os.Args[1] {
		case "bootstrap":
			runBootstrapDispatch(os.Args[2:])
			return
		case "version", "--version", "-v":
			printVersion()
			return
		case "vms":
			printVMs()
			return
		}
	}

	// Print module versions for build verification
	if info, ok := debug.ReadBuildInfo(); ok {
		for _, dep := range info.Deps {
			if dep.Path == "github.com/luxfi/node" {
				fmt.Fprintf(os.Stderr, "[BUILD] luxfi/node module: %s\n", dep.Version)
				break
			}
		}
	}

	// Default: run as a full node.
	runNode()
}

func printVersion() {
	versions := nodeversion.GetVersions()
	fmt.Printf("zood %s (luxd %s)\n", "0.2.0", versions.String())
}

// printVMs lists the native and inherited VMs registered into zood.
// The 3-VM triumvirate (Zoo EVM, Zoo DEX, Zoo FHE) is wired locally;
// the 8 other Lux optional VMs come via github.com/luxfi/node.
func printVMs() {
	fmt.Println("zood registered VMs:")
	fmt.Printf("  zoo-evm     %s   GPU EVM with Zoo precompiles  luxfi/cevm + luxcpp/cevm\n", vm.EVMID.String())
	fmt.Printf("  zoo-dex     %s   CLOB matching engine          luxcpp/dex bindings\n", vm.DEXVMID.String())
	fmt.Printf("  zoo-fhe     %s   CKKS/TFHE encrypted compute   luxcpp/fhe bindings\n", vm.FHEVMID.String())
	fmt.Println("  --- inherited from luxfi/node ---")
	fmt.Println("  aivm       (A-Chain)   AI inference")
	fmt.Println("  bridgevm   (B-Chain)   Cross-chain bridge")
	fmt.Println("  graphvm    (G-Chain)   Graph database")
	fmt.Println("  identityvm (I-Chain)   DID/VC")
	fmt.Println("  keyvm      (K-Chain)   PQ key management")
	fmt.Println("  oraclevm   (O-Chain)   Oracle feeds")
	fmt.Println("  quantumvm  (Q-Chain)   PQ consensus coordination")
	fmt.Println("  relayvm    (R-Chain)   Cross-chain message relay")
	fmt.Println("  thresholdvm(T-Chain)   Threshold MPC + FHE")
	fmt.Println("  zkvm       (Z-Chain)   Zero-knowledge proofs")
}

func runNode() {
	fs := config.BuildFlagSet()
	fs.Int("zap-port", defaultZAPPort, "TCP port for ZAP binary protocol listener (0 to disable)")
	v, err := config.BuildViper(fs, os.Args[1:])

	if errors.Is(err, pflag.ErrHelp) {
		os.Exit(0)
	}
	if err != nil {
		fmt.Printf("couldn't configure flags: %s\n", err)
		os.Exit(1)
	}

	if v.GetBool(config.VersionJSONKey) {
		versions := nodeversion.GetVersions()
		jsonBytes, err := json.MarshalIndent(versions, "", "  ")
		if err != nil {
			fmt.Printf("couldn't marshal versions: %s\n", err)
			os.Exit(1)
		}
		fmt.Println(string(jsonBytes))
		os.Exit(0)
	}

	if v.GetBool(config.VersionKey) {
		fmt.Println(nodeversion.GetVersions().String())
		os.Exit(0)
	}

	nodeConfig, err := config.GetNodeConfig(v)
	if err != nil {
		fmt.Printf("couldn't load node config: %s\n", err)
		os.Exit(1)
	}

	// Install ourselves as EVM, DEX, and C-Chain VM plugins before the node starts.
	// The C-Chain VM (evm.ID) must also be installed as a plugin — newer Lux versions
	// launch ALL chain VMs as subprocesses, including the built-in C-chain.
	for _, vmID := range []string{
		vm.EVMID.String(),    // Zoo EVM
		vm.DEXVMID.String(),  // Zoo DEX
		vm.FHEVMID.String(),  // Zoo FHE
		vm.StandardEVMID().String(),
	} {
		if err := installPlugin(nodeConfig.PluginDir, vmID); err != nil {
			fmt.Printf("couldn't install VM plugin %s: %s\n", vmID[:8], err)
			os.Exit(1)
		}
	}

	if term.IsTerminal(int(os.Stdout.Fd())) {
		fmt.Print(header)
	}

	nodeApp, err := app.New(nodeConfig)
	if err != nil {
		fmt.Printf("couldn't start node: %s\n", err)
		os.Exit(1)
	}

	// Start ZAP TCP listener for binary protocol access to DEX operations.
	// The ZAP port defaults to 9633 and can be overridden via --zap-port flag or ZAP_PORT env.
	zapPort := zapPortFromEnv()
	if p := v.GetInt("zap-port"); p > 0 {
		zapPort = p
	}

	httpPort := nodeConfig.HTTPPort
	evmRPCURL := fmt.Sprintf("http://127.0.0.1:%d/ext/bc/zooevm/rpc", httpPort)

	zapLogger := slog.Default()
	zapNode, zapErr := startZAP(zapPort, evmRPCURL, zapLogger)
	if zapErr != nil {
		fmt.Printf("warning: couldn't start ZAP listener: %s\n", zapErr)
		// Non-fatal — the node still runs without ZAP.
	}

	exitCode := app.Run(nodeApp)

	// Graceful shutdown of ZAP listener.
	if zapNode != nil {
		zapNode.Stop()
	}

	os.Exit(exitCode)
}

// installPlugin creates a symlink of this binary in the plugin
// directory under the given VM ID name, so the node's VMRegistry discovers it.
func installPlugin(pluginDir string, vmIDStr string) error {
	if pluginDir == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			return fmt.Errorf("couldn't get home dir: %w", err)
		}
		pluginDir = filepath.Join(home, ".lux", "plugins")
	}

	if err := os.MkdirAll(pluginDir, 0700); err != nil {
		return fmt.Errorf("couldn't create plugin dir: %w", err)
	}

	pluginPath := filepath.Join(pluginDir, vmIDStr)

	exe, err := os.Executable()
	if err != nil {
		return fmt.Errorf("couldn't get executable path: %w", err)
	}
	exe, err = filepath.EvalSymlinks(exe)
	if err != nil {
		return fmt.Errorf("couldn't resolve executable path: %w", err)
	}

	// Atomic install: create temp symlink, then rename (atomic on POSIX).
	tmpPath := pluginPath + ".tmp." + strconv.Itoa(os.Getpid())
	_ = os.Remove(tmpPath)

	if err := os.Symlink(exe, tmpPath); err != nil {
		// Fallback to hard link if symlink fails (e.g., cross-device).
		if err := os.Link(exe, tmpPath); err != nil {
			return fmt.Errorf("couldn't link plugin binary: %w", err)
		}
	}

	if err := os.Rename(tmpPath, pluginPath); err != nil {
		_ = os.Remove(tmpPath)
		return fmt.Errorf("couldn't install plugin atomically: %w", err)
	}

	return nil
}
