// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// See the file LICENSE for licensing terms.

//go:build bootstrap
// +build bootstrap

package main

// runBootstrapDispatch routes to the full bootstrap implementation
// when built with -tags=bootstrap. See bootstrap.go.
func runBootstrapDispatch(args []string) {
	runBootstrap(args)
}
