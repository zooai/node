// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// See the file LICENSE for licensing terms.

package vm

import (
	"testing"
)

func TestVMID_NonEmpty(t *testing.T) {
	id := VMID()
	if id == "" {
		t.Fatal("VMID() returned empty string")
	}
	t.Logf("VMID: %s", id)
}

func TestVMID_Consistent(t *testing.T) {
	id1 := VMID()
	id2 := VMID()
	if id1 != id2 {
		t.Fatalf("VMID() not consistent: %q != %q", id1, id2)
	}
}

func TestVMID_ValidFormat(t *testing.T) {
	id := VMID()
	// VM IDs are CB58-encoded 32-byte IDs. They should be alphanumeric
	// (base58) and typically 40-50 characters.
	if len(id) < 30 || len(id) > 60 {
		t.Errorf("VMID length %d looks wrong (expected 30-60 chars for CB58): %q", len(id), id)
	}

	// Verify all characters are valid base58 (no 0, O, I, l).
	for _, c := range id {
		if c == '0' || c == 'O' || c == 'I' || c == 'l' {
			t.Errorf("VMID contains invalid base58 character %q: %s", c, id)
		}
	}
}
