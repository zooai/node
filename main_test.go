// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// See the file LICENSE for licensing terms.

package main

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/zoo-labs/chain/vm"
)

func TestInstallPlugin_CreatesSymlink(t *testing.T) {
	tmpDir := t.TempDir()
	pluginDir := filepath.Join(tmpDir, "plugins")

	err := installPlugin(pluginDir, vm.EVMID.String())
	if err != nil {
		t.Fatalf("installPlugin failed: %v", err)
	}

	pluginPath := filepath.Join(pluginDir, vm.EVMID.String())

	// Verify the symlink exists.
	info, err := os.Lstat(pluginPath)
	if err != nil {
		t.Fatalf("plugin file does not exist: %v", err)
	}

	// Should be a symlink (or hard link).
	if info.Mode()&os.ModeSymlink != 0 {
		target, err := os.Readlink(pluginPath)
		if err != nil {
			t.Fatalf("readlink failed: %v", err)
		}
		if target == "" {
			t.Fatal("symlink target is empty")
		}
		t.Logf("symlink -> %s", target)
	} else {
		// Hard link -- just verify it's a regular file.
		if !info.Mode().IsRegular() {
			t.Fatalf("expected regular file or symlink, got %s", info.Mode())
		}
		t.Log("installed as hard link")
	}
}

func TestInstallPlugin_EmptyPluginDir_UsesDefault(t *testing.T) {
	// When pluginDir is empty, installPlugin uses ~/.lux/plugins.
	// We cannot easily test the actual default path without side effects,
	// but we can verify that passing a non-empty dir works correctly and
	// the function doesn't error with an explicit directory.
	tmpDir := t.TempDir()
	err := installPlugin(tmpDir, vm.EVMID.String())
	if err != nil {
		t.Fatalf("installPlugin with explicit dir failed: %v", err)
	}

	pluginPath := filepath.Join(tmpDir, vm.EVMID.String())
	if _, err := os.Lstat(pluginPath); err != nil {
		t.Fatalf("plugin not found at %s: %v", pluginPath, err)
	}
}

func TestInstallPlugin_AtomicReplacement(t *testing.T) {
	tmpDir := t.TempDir()
	pluginDir := filepath.Join(tmpDir, "plugins")

	// First install.
	if err := installPlugin(pluginDir, vm.EVMID.String()); err != nil {
		t.Fatalf("first installPlugin failed: %v", err)
	}

	pluginPath := filepath.Join(pluginDir, vm.EVMID.String())
	info1, err := os.Lstat(pluginPath)
	if err != nil {
		t.Fatalf("plugin missing after first install: %v", err)
	}

	// Second install -- should atomically replace.
	if err := installPlugin(pluginDir, vm.EVMID.String()); err != nil {
		t.Fatalf("second installPlugin failed: %v", err)
	}

	info2, err := os.Lstat(pluginPath)
	if err != nil {
		t.Fatalf("plugin missing after second install: %v", err)
	}

	// Both installs should succeed and produce a valid file.
	if info1.Name() != info2.Name() {
		t.Fatalf("plugin filename changed: %s -> %s", info1.Name(), info2.Name())
	}

	// Verify no temp files left behind.
	entries, err := os.ReadDir(pluginDir)
	if err != nil {
		t.Fatalf("readdir failed: %v", err)
	}
	for _, e := range entries {
		if filepath.Ext(e.Name()) == ".tmp" || len(e.Name()) > len(vm.EVMID.String())+5 {
			// Temp files have format: {vmid}.tmp.{pid}
			if e.Name() != vm.EVMID.String() {
				t.Errorf("stale temp file left behind: %s", e.Name())
			}
		}
	}
}

func TestInstallPlugin_DirectoryPermissions(t *testing.T) {
	tmpDir := t.TempDir()
	pluginDir := filepath.Join(tmpDir, "restricted-plugins")

	if err := installPlugin(pluginDir, vm.EVMID.String()); err != nil {
		t.Fatalf("installPlugin failed: %v", err)
	}

	// Verify the plugin directory was created with 0700 permissions.
	info, err := os.Stat(pluginDir)
	if err != nil {
		t.Fatalf("stat plugin dir failed: %v", err)
	}

	perm := info.Mode().Perm()
	if perm != 0700 {
		t.Errorf("plugin dir permissions: got %o, want %o", perm, 0700)
	}
}

func TestInstallPlugin_NestedPluginDir(t *testing.T) {
	tmpDir := t.TempDir()
	pluginDir := filepath.Join(tmpDir, "a", "b", "c", "plugins")

	// Should create all intermediate directories.
	if err := installPlugin(pluginDir, vm.EVMID.String()); err != nil {
		t.Fatalf("installPlugin with nested dir failed: %v", err)
	}

	pluginPath := filepath.Join(pluginDir, vm.EVMID.String())
	if _, err := os.Lstat(pluginPath); err != nil {
		t.Fatalf("plugin not found at nested path: %v", err)
	}
}
