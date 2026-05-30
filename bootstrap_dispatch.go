// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// See the file LICENSE for licensing terms.

//go:build !bootstrap
// +build !bootstrap

package main

import (
	"fmt"
	"os"
)

// runBootstrapDispatch is the default no-op bootstrap entry point.
// The full bootstrap implementation lives in bootstrap.go behind the
// "bootstrap" build tag because it pulls in luxfi/sdk wallet packages
// that are heavier (and currently in flux). Build the bootstrap-capable
// binary with: go build -tags=bootstrap -o zood-boot .
func runBootstrapDispatch(args []string) {
	fmt.Fprintln(os.Stderr, "zood: bootstrap subcommand not built into this binary")
	fmt.Fprintln(os.Stderr, "      rebuild with: go build -tags=bootstrap")
	os.Exit(2)
}
