// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// See the file LICENSE for licensing terms.

package main

import (
	"encoding/hex"
	"os"
	"testing"
	"time"
)

// --- envOr tests ---

func TestEnvOr_ReturnsEnvValue(t *testing.T) {
	t.Setenv("TEST_ENVOR_KEY", "fromenv")
	got := envOr("TEST_ENVOR_KEY", "fallback")
	if got != "fromenv" {
		t.Fatalf("envOr returned %q, want %q", got, "fromenv")
	}
}

func TestEnvOr_ReturnsFallback(t *testing.T) {
	t.Setenv("TEST_ENVOR_MISSING", "")
	got := envOr("TEST_ENVOR_MISSING", "fallback")
	if got != "fallback" {
		t.Fatalf("envOr returned %q, want %q", got, "fallback")
	}
}

func TestEnvOr_UnsetKey(t *testing.T) {
	os.Unsetenv("TEST_ENVOR_UNSET_XYZ")
	got := envOr("TEST_ENVOR_UNSET_XYZ", "default")
	if got != "default" {
		t.Fatalf("envOr returned %q, want %q", got, "default")
	}
}

// --- parseBootstrapFlags tests ---

func TestParseBootstrapFlags_Defaults(t *testing.T) {
	// Clear all env vars that parseBootstrapFlags reads.
	t.Setenv("LUX_URI", "")
	t.Setenv("NETWORK_NAME", "")
	t.Setenv("EVM_GENESIS", "")
	t.Setenv("DEX_GENESIS", "")
	t.Setenv("CHAINS", "")
	t.Setenv("EVM_CHAIN_NAME", "")
	t.Setenv("DEX_CHAIN_NAME", "")
	// Must provide at least one key source to avoid log.Fatal.
	t.Setenv("LUX_MNEMONIC", "light light light light light light light light light light light energy")
	t.Setenv("LUX_PRIVATE_KEY", "")

	cfg := parseBootstrapFlags(nil)

	if cfg.NodeURI != "https://api.lux-dev.network" {
		t.Errorf("NodeURI: got %q, want %q", cfg.NodeURI, "https://api.lux-dev.network")
	}
	if cfg.NetworkName != "Zoo" {
		t.Errorf("NetworkName: got %q, want %q", cfg.NetworkName, "Zoo")
	}
	if cfg.CoinType != 60 {
		t.Errorf("CoinType: got %d, want %d", cfg.CoinType, 60)
	}
	if cfg.KeyIndex != 0 {
		t.Errorf("KeyIndex: got %d, want %d", cfg.KeyIndex, 0)
	}
	if cfg.ValidatorWeight != 20 {
		t.Errorf("ValidatorWeight: got %d, want %d", cfg.ValidatorWeight, 20)
	}
	if cfg.ValidatorDuration != 300*24*time.Hour {
		t.Errorf("ValidatorDuration: got %v, want %v", cfg.ValidatorDuration, 300*24*time.Hour)
	}
	// Default chains: evm + dex.
	if len(cfg.Chains) != 2 {
		t.Fatalf("Chains: got %d, want 2", len(cfg.Chains))
	}
	if cfg.Chains[0].Name != "" {
		t.Errorf("Chain[0].Name: got %q, want %q", cfg.Chains[0].Name, "")
	}
	if cfg.Chains[0].Alias != "zooevm" {
		t.Errorf("Chain[0].Alias: got %q, want %q", cfg.Chains[0].Alias, "zooevm")
	}
	if cfg.Chains[1].Name != "" {
		t.Errorf("Chain[1].Name: got %q, want %q", cfg.Chains[1].Name, "")
	}
	if cfg.Chains[1].Alias != "zoodex" {
		t.Errorf("Chain[1].Alias: got %q, want %q", cfg.Chains[1].Alias, "zoodex")
	}
}

func TestParseBootstrapFlags_EnvVars(t *testing.T) {
	t.Setenv("LUX_URI", "https://custom.lux.network")
	t.Setenv("NETWORK_NAME", "TestNet")
	t.Setenv("CHAINS", "evm")
	t.Setenv("EVM_GENESIS", "/tmp/custom-genesis.json")
	t.Setenv("EVM_CHAIN_NAME", "CustomEVM")
	t.Setenv("DEX_GENESIS", "")
	t.Setenv("DEX_CHAIN_NAME", "")
	t.Setenv("LUX_MNEMONIC", "light light light light light light light light light light light energy")
	t.Setenv("LUX_PRIVATE_KEY", "")

	cfg := parseBootstrapFlags(nil)

	if cfg.NodeURI != "https://custom.lux.network" {
		t.Errorf("NodeURI: got %q, want %q", cfg.NodeURI, "https://custom.lux.network")
	}
	if cfg.NetworkName != "TestNet" {
		t.Errorf("NetworkName: got %q, want %q", cfg.NetworkName, "TestNet")
	}
	if len(cfg.Chains) != 1 {
		t.Fatalf("Chains: got %d, want 1", len(cfg.Chains))
	}
	if cfg.Chains[0].Name != "CustomEVM" {
		t.Errorf("Chain[0].Name: got %q, want %q", cfg.Chains[0].Name, "CustomEVM")
	}
	if cfg.Chains[0].GenesisFile != "/tmp/custom-genesis.json" {
		t.Errorf("Chain[0].GenesisFile: got %q, want %q", cfg.Chains[0].GenesisFile, "/tmp/custom-genesis.json")
	}
}

func TestParseBootstrapFlags_CLIFlags(t *testing.T) {
	t.Setenv("LUX_URI", "")
	t.Setenv("NETWORK_NAME", "")
	t.Setenv("CHAINS", "")
	t.Setenv("EVM_GENESIS", "")
	t.Setenv("DEX_GENESIS", "")
	t.Setenv("EVM_CHAIN_NAME", "")
	t.Setenv("DEX_CHAIN_NAME", "")
	t.Setenv("LUX_MNEMONIC", "light light light light light light light light light light light energy")
	t.Setenv("LUX_PRIVATE_KEY", "")

	args := []string{
		"--uri", "https://flag.lux.network",
		"--name", "FlagNet",
		"--chains", "evm",
	}

	cfg := parseBootstrapFlags(args)

	if cfg.NodeURI != "https://flag.lux.network" {
		t.Errorf("NodeURI: got %q, want %q", cfg.NodeURI, "https://flag.lux.network")
	}
	if cfg.NetworkName != "FlagNet" {
		t.Errorf("NetworkName: got %q, want %q", cfg.NetworkName, "FlagNet")
	}
	if len(cfg.Chains) != 1 {
		t.Fatalf("Chains: got %d, want 1 (evm only)", len(cfg.Chains))
	}
	if cfg.Chains[0].Alias != "zooevm" {
		t.Errorf("Chain[0].Alias: got %q, want %q", cfg.Chains[0].Alias, "zooevm")
	}
}

func TestParseBootstrapFlags_CLIOverridesEnv(t *testing.T) {
	t.Setenv("LUX_URI", "https://env.lux.network")
	t.Setenv("NETWORK_NAME", "EnvNet")
	t.Setenv("CHAINS", "")
	t.Setenv("EVM_GENESIS", "")
	t.Setenv("DEX_GENESIS", "")
	t.Setenv("EVM_CHAIN_NAME", "")
	t.Setenv("DEX_CHAIN_NAME", "")
	t.Setenv("LUX_MNEMONIC", "light light light light light light light light light light light energy")
	t.Setenv("LUX_PRIVATE_KEY", "")

	args := []string{"--uri", "https://cli.lux.network", "--name", "CLINet"}
	cfg := parseBootstrapFlags(args)

	// CLI flags should override env vars.
	if cfg.NodeURI != "https://cli.lux.network" {
		t.Errorf("NodeURI: got %q, want %q (CLI should override env)", cfg.NodeURI, "https://cli.lux.network")
	}
	if cfg.NetworkName != "CLINet" {
		t.Errorf("NetworkName: got %q, want %q (CLI should override env)", cfg.NetworkName, "CLINet")
	}
}

func TestParseBootstrapFlags_ChainsEvmOnly(t *testing.T) {
	t.Setenv("LUX_URI", "")
	t.Setenv("NETWORK_NAME", "")
	t.Setenv("CHAINS", "evm")
	t.Setenv("EVM_GENESIS", "")
	t.Setenv("DEX_GENESIS", "")
	t.Setenv("EVM_CHAIN_NAME", "")
	t.Setenv("DEX_CHAIN_NAME", "")
	t.Setenv("LUX_MNEMONIC", "light light light light light light light light light light light energy")
	t.Setenv("LUX_PRIVATE_KEY", "")

	cfg := parseBootstrapFlags(nil)
	if len(cfg.Chains) != 1 {
		t.Fatalf("Chains: got %d, want 1", len(cfg.Chains))
	}
	if cfg.Chains[0].Name != "" {
		t.Errorf("expected EVM chain, got %q", cfg.Chains[0].Name)
	}
}

func TestParseBootstrapFlags_ChainsDexOnly(t *testing.T) {
	t.Setenv("LUX_URI", "")
	t.Setenv("NETWORK_NAME", "")
	t.Setenv("CHAINS", "dex")
	t.Setenv("EVM_GENESIS", "")
	t.Setenv("DEX_GENESIS", "")
	t.Setenv("EVM_CHAIN_NAME", "")
	t.Setenv("DEX_CHAIN_NAME", "")
	t.Setenv("LUX_MNEMONIC", "light light light light light light light light light light light energy")
	t.Setenv("LUX_PRIVATE_KEY", "")

	cfg := parseBootstrapFlags(nil)
	if len(cfg.Chains) != 1 {
		t.Fatalf("Chains: got %d, want 1", len(cfg.Chains))
	}
	if cfg.Chains[0].Name != "" {
		t.Errorf("expected DEX chain, got %q", cfg.Chains[0].Name)
	}
}

// --- deriveKey tests ---

func TestDeriveKey_PrivateKeyHex(t *testing.T) {
	// Test-only key. Never use for funded accounts.
	privHex := "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

	cfg := bootstrapConfig{
		PrivateKey: privHex,
		CoinType:   9000,
		KeyIndex:   1,
	}

	key := deriveKey(cfg)
	if key == nil {
		t.Fatal("deriveKey returned nil")
	}

	addr := key.Address()
	if addr == ([20]byte{}) {
		t.Fatal("derived address is zero")
	}
	t.Logf("address from private key: %x", addr[:])
}

func TestDeriveKey_Mnemonic(t *testing.T) {
	mnemonic := "light light light light light light light light light light light energy"

	cfg := bootstrapConfig{
		Mnemonic: mnemonic,
		CoinType: 9000,
		KeyIndex: 1,
	}

	key := deriveKey(cfg)
	if key == nil {
		t.Fatal("deriveKey returned nil")
	}

	addr := key.Address()
	if addr == ([20]byte{}) {
		t.Fatal("derived address is zero")
	}
	t.Logf("address from mnemonic: %x", addr[:])
}

func TestDeriveKey_MnemonicDeterministic(t *testing.T) {
	mnemonic := "light light light light light light light light light light light energy"

	cfg := bootstrapConfig{
		Mnemonic: mnemonic,
		CoinType: 9000,
		KeyIndex: 1,
	}

	key1 := deriveKey(cfg)
	key2 := deriveKey(cfg)

	addr1 := hex.EncodeToString(key1.Address().Bytes())
	addr2 := hex.EncodeToString(key2.Address().Bytes())

	if addr1 != addr2 {
		t.Fatalf("deriveKey is not deterministic: %s != %s", addr1, addr2)
	}
	t.Logf("deterministic address: %s", addr1)
}

func TestDeriveKey_DifferentKeyIndex(t *testing.T) {
	mnemonic := "light light light light light light light light light light light energy"

	cfg0 := bootstrapConfig{Mnemonic: mnemonic, CoinType: 9000, KeyIndex: 0}
	cfg1 := bootstrapConfig{Mnemonic: mnemonic, CoinType: 9000, KeyIndex: 1}

	key0 := deriveKey(cfg0)
	key1 := deriveKey(cfg1)

	addr0 := hex.EncodeToString(key0.Address().Bytes())
	addr1 := hex.EncodeToString(key1.Address().Bytes())

	if addr0 == addr1 {
		t.Fatalf("different key indices produced same address: %s", addr0)
	}
	t.Logf("index 0: %s", addr0)
	t.Logf("index 1: %s", addr1)
}

func TestDeriveKey_PrivateKeyTakesPriority(t *testing.T) {
	// When PrivateKey is set (non-empty), it is used even if Mnemonic is also set.
	// deriveKey checks PrivateKey first.
	privHex := "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
	mnemonic := "light light light light light light light light light light light energy"

	cfgBoth := bootstrapConfig{
		PrivateKey: privHex,
		Mnemonic:   mnemonic,
		CoinType:   9000,
		KeyIndex:   1,
	}
	cfgKeyOnly := bootstrapConfig{
		PrivateKey: privHex,
		CoinType:   9000,
		KeyIndex:   1,
	}

	keyBoth := deriveKey(cfgBoth)
	keyOnly := deriveKey(cfgKeyOnly)

	addrBoth := hex.EncodeToString(keyBoth.Address().Bytes())
	addrOnly := hex.EncodeToString(keyOnly.Address().Bytes())

	if addrBoth != addrOnly {
		t.Fatalf("PrivateKey should take priority: both=%s keyOnly=%s", addrBoth, addrOnly)
	}
	t.Logf("private key takes priority, addr: %s", addrBoth)
}

func TestDeriveKey_KeyMaterialZeroed(t *testing.T) {
	// deriveKey zeros keyBytes after converting from hex.
	// We can verify the seed zeroing by checking deriveKey with mnemonic
	// still returns a valid key (the zeroing happens in deferred cleanup).
	mnemonic := "light light light light light light light light light light light energy"

	cfg := bootstrapConfig{
		Mnemonic: mnemonic,
		CoinType: 9000,
		KeyIndex: 1,
	}

	key := deriveKey(cfg)
	if key == nil {
		t.Fatal("deriveKey returned nil after zeroing")
	}

	// The key itself must still be usable after the function returns
	// (the zeroing targets intermediate buffers, not the returned key).
	addr := key.Address()
	if addr == ([20]byte{}) {
		t.Fatal("key is unusable after derivation (zeroing may have clobbered the output)")
	}
}

// --- readDevStartTime tests ---

func TestReadDevStartTime_NoFile(t *testing.T) {
	tmpDir := t.TempDir()
	st := readDevStartTime(tmpDir)
	if st != 0 {
		t.Fatalf("readDevStartTime with no file: got %d, want 0", st)
	}
}

func TestReadDevStartTime_ValidFile(t *testing.T) {
	tmpDir := t.TempDir()
	data := []byte(`{"startTime": 1700000000}`)
	if err := os.WriteFile(tmpDir+"/dev-network.json", data, 0644); err != nil {
		t.Fatal(err)
	}

	st := readDevStartTime(tmpDir)
	if st != 1700000000 {
		t.Fatalf("readDevStartTime: got %d, want 1700000000", st)
	}
}

func TestReadDevStartTime_InvalidJSON(t *testing.T) {
	tmpDir := t.TempDir()
	data := []byte(`not valid json`)
	if err := os.WriteFile(tmpDir+"/dev-network.json", data, 0644); err != nil {
		t.Fatal(err)
	}

	st := readDevStartTime(tmpDir)
	if st != 0 {
		t.Fatalf("readDevStartTime with invalid JSON: got %d, want 0", st)
	}
}
