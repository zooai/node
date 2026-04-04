// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// See the file LICENSE for licensing terms.

package main

import (
	"encoding/hex"
	"fmt"
	"os"
	"testing"

	"github.com/luxfi/crypto/secp256k1"
)

func TestLoadDevKey_Mnemonic(t *testing.T) {
	// Zoo test mnemonic (NOT production — unit tests only)
	mnemonic := "light light light light light light light light light light light energy"
	t.Setenv("MNEMONIC", mnemonic)
	t.Setenv("PRIVATE_KEY", "") // ensure priority works

	key, err := loadDevKey()
	if err != nil {
		t.Fatalf("loadDevKey failed: %v", err)
	}

	secpAddr := key.Address()
	ethAddr := secp256k1.PubkeyToAddress(key.ToECDSA().PublicKey)

	// The secp256k1 address and ETH address MUST differ for the same key.
	// This is the core bug: if they're the same, the fix is wrong.
	secpHex := hex.EncodeToString(secpAddr[:])
	ethHex := hex.EncodeToString(ethAddr[:])

	if secpHex == ethHex {
		t.Fatalf("secp256k1 and ETH addresses must differ for the same key\n  secp: %s\n  eth:  %s", secpHex, ethHex)
	}

	t.Logf("secp256k1 addr (P-Chain): %s", secpHex)
	t.Logf("ETH addr      (C-Chain): 0x%s", ethHex)
}

func TestLoadDevKey_PrivateKey(t *testing.T) {
	// Use LIGHT_MNEMONIC for all tests
	t.Setenv("LIGHT_MNEMONIC", "light light light light light light light light light light light energy")
	t.Setenv("MNEMONIC", "")
	t.Setenv("PRIVATE_KEY", "")

	key, err := loadDevKey()
	if err != nil {
		t.Fatalf("loadDevKey failed: %v", err)
	}

	secpAddr := key.Address()
	ethAddr := secp256k1.PubkeyToAddress(key.ToECDSA().PublicKey)

	// Verify the ETH address is non-empty
	ethHex := hex.EncodeToString(ethAddr[:])
	if ethHex == "0000000000000000000000000000000000000000" {
		t.Fatal("ETH address is zero")
	}
	t.Logf("ETH addr from LIGHT_MNEMONIC: 0x%s", ethHex)

	// secp256k1 address must be different
	secpHex := hex.EncodeToString(secpAddr[:])
	if secpHex == ethHex {
		t.Fatal("secp256k1 and ETH addresses must differ")
	}

	t.Logf("secp256k1 addr (P-Chain): %s", secpHex)
	t.Logf("ETH addr      (C-Chain): 0x%s", ethHex)
}

func TestLoadDevKey_NoEnv(t *testing.T) {
	t.Setenv("MNEMONIC", "")
	t.Setenv("PRIVATE_KEY", "")
	t.Setenv("LIGHT_MNEMONIC", "")

	_, err := loadDevKey()
	if err == nil {
		t.Fatal("expected error when no env vars set")
	}
}

func TestLoadDevKey_Priority(t *testing.T) {
	// When both are set, LUX_MNEMONIC should take priority
	mnemonic := "light light light light light light light light light light light energy"
	privHex := "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

	t.Setenv("MNEMONIC", mnemonic)
	t.Setenv("PRIVATE_KEY", privHex)

	key, err := loadDevKey()
	if err != nil {
		t.Fatalf("loadDevKey failed: %v", err)
	}

	// Verify it used the mnemonic, not the private key
	ethAddr := secp256k1.PubkeyToAddress(key.ToECDSA().PublicKey)
	ethHex := hex.EncodeToString(ethAddr[:])

	// Anvil #0 address -- if we get this, mnemonic was NOT used
	anvilAddr := "f39fd6e51aad88f6f4ce6ab8827279cfffb92266"
	if ethHex == anvilAddr {
		t.Fatal("LUX_MNEMONIC should take priority over LUX_PRIVATE_KEY")
	}

	t.Logf("mnemonic key ETH addr: 0x%s (not anvil #0: 0x%s)", ethHex, anvilAddr)
}

func TestDevCChainGenesis(t *testing.T) {
	var addr [20]byte
	copy(addr[:], []byte{0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89,
		0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB})

	genesis := devCChainGenesis(addr)

	// Verify the address appears in the genesis
	addrHex := fmt.Sprintf("%x", addr)
	if len(genesis) == 0 {
		t.Fatal("empty genesis")
	}

	// The address should appear in the alloc section
	if !contains(genesis, addrHex) {
		t.Fatalf("genesis does not contain address %s", addrHex)
	}

	// Verify standard test accounts are also present
	testAccounts := []string{
		"f39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
		"70997970C51812dc3A010C7d01b50e0d17dc79C8",
	}
	for _, acct := range testAccounts {
		if !contains(genesis, acct) {
			t.Fatalf("genesis does not contain test account %s", acct)
		}
	}
}

func contains(s, substr string) bool {
	return len(s) > 0 && len(substr) > 0 && stringContains(s, substr)
}

func stringContains(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}

// Ensure test imports are used
var _ = os.Getenv
