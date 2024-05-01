// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// See the file LICENSE for licensing terms.

package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"os"
	"strconv"

	"github.com/luxfi/zap"
)

// ZAP message types for DEX operations.
// Upper 8 bits of the flags field identify the message type.
const (
	MsgQuote     uint16 = 1
	MsgSwap      uint16 = 2
	MsgOrderbook uint16 = 3
)

// ZAP request object field offsets.
// Each request is a fixed-size struct with text fields for the JSON-RPC params.
const (
	// Common request fields
	fieldMethod  = 0  // text: JSON-RPC method name (offset 0, 8 bytes: ptr+len)
	fieldParams  = 8  // text: JSON-RPC params as JSON string (offset 8, 8 bytes)
	fieldReqSize = 16 // total fixed size
)

// ZAP response object field offsets.
const (
	fieldStatus   = 0  // uint32: HTTP status code
	fieldBody     = 4  // text: response body (JSON)
	fieldRespSize = 12 // total fixed size
)

// defaultZAPPort is the default TCP port for the ZAP listener.
const defaultZAPPort = 9633

// zapPortFromEnv returns the ZAP port from the ZAP_PORT env var, or the default.
func zapPortFromEnv() int {
	if s := os.Getenv("ZAP_PORT"); s != "" {
		if p, err := strconv.Atoi(s); err == nil && p > 0 && p < 65536 {
			return p
		}
	}
	return defaultZAPPort
}

// startZAP creates and starts a ZAP node that proxies DEX requests to the local EVM RPC.
// It returns the ZAP node (caller must call Stop) or an error.
func startZAP(port int, evmRPCURL string, logger *slog.Logger) (*zap.Node, error) {
	hostname, _ := os.Hostname()
	if hostname == "" {
		hostname = "zood"
	}

	node := zap.NewNode(zap.NodeConfig{
		NodeID:      fmt.Sprintf("zood-%s-%d", hostname, os.Getpid()),
		ServiceType: "_zoodty._tcp",
		Port:        port,
		NoDiscovery: true, // Server-only — no mDNS needed for a node listener
		Logger:      logger,
	})

	proxy := &evmProxy{
		rpcURL: evmRPCURL,
		client: &http.Client{},
	}

	// Register DEX operation handlers.
	// Each handler decodes the ZAP request, builds a JSON-RPC call, proxies to the
	// local EVM RPC, and returns the result as a ZAP response.

	node.Handle(MsgQuote, proxy.handleRPC("dex.GetQuote"))
	node.Handle(MsgSwap, proxy.handleRPC("dex.Swap"))
	node.Handle(MsgOrderbook, proxy.handleRPC("dex.GetOrderbook"))

	if err := node.Start(); err != nil {
		return nil, fmt.Errorf("zap: failed to start: %w", err)
	}

	logger.Info("ZAP listener started",
		"port", port,
		"service", "_zoodty._tcp",
		"rpc_target", evmRPCURL,
	)

	return node, nil
}

// evmProxy proxies ZAP requests to a local JSON-RPC endpoint.
type evmProxy struct {
	rpcURL string
	client *http.Client
}

// handleRPC returns a ZAP handler that proxies to the given JSON-RPC method.
// If the incoming ZAP message contains a method field, that overrides the default.
// If it contains a params field, those are used as JSON-RPC params.
func (p *evmProxy) handleRPC(defaultMethod string) zap.Handler {
	return func(ctx context.Context, from string, msg *zap.Message) (*zap.Message, error) {
		root := msg.Root()

		// Read method from the ZAP message, fall back to default.
		method := root.Text(fieldMethod)
		if method == "" {
			method = defaultMethod
		}

		// Read params from the ZAP message (raw JSON string).
		paramsJSON := root.Text(fieldParams)
		var params json.RawMessage
		if paramsJSON != "" {
			params = json.RawMessage(paramsJSON)
		} else {
			params = json.RawMessage("[]")
		}

		// Build JSON-RPC 2.0 request.
		rpcReq := struct {
			JSONRPC string          `json:"jsonrpc"`
			ID      int             `json:"id"`
			Method  string          `json:"method"`
			Params  json.RawMessage `json:"params"`
		}{
			JSONRPC: "2.0",
			ID:      1,
			Method:  method,
			Params:  params,
		}

		reqBody, err := json.Marshal(rpcReq)
		if err != nil {
			return buildErrorResponse(400, fmt.Sprintf("failed to marshal request: %v", err)), nil
		}

		// POST to the local EVM RPC.
		httpReq, err := http.NewRequestWithContext(ctx, http.MethodPost, p.rpcURL, bytes.NewReader(reqBody))
		if err != nil {
			return buildErrorResponse(500, fmt.Sprintf("failed to create request: %v", err)), nil
		}
		httpReq.Header.Set("Content-Type", "application/json")

		resp, err := p.client.Do(httpReq)
		if err != nil {
			return buildErrorResponse(502, fmt.Sprintf("rpc call failed: %v", err)), nil
		}
		defer resp.Body.Close()

		body, err := io.ReadAll(io.LimitReader(resp.Body, 10*1024*1024)) // 10MB max
		if err != nil {
			return buildErrorResponse(502, fmt.Sprintf("failed to read response: %v", err)), nil
		}

		return buildResponse(uint32(resp.StatusCode), string(body)), nil
	}
}

// buildResponse constructs a ZAP response message with a status code and body.
func buildResponse(status uint32, body string) *zap.Message {
	b := zap.NewBuilder(256 + len(body))
	obj := b.StartObject(fieldRespSize)
	obj.SetUint32(fieldStatus, status)
	obj.SetText(fieldBody, body)
	obj.FinishAsRoot()

	msg, err := zap.Parse(b.Finish())
	if err != nil {
		// This should never happen — we just built the message.
		return nil
	}
	return msg
}

// buildErrorResponse constructs a ZAP response with a JSON-RPC error body.
func buildErrorResponse(status uint32, errMsg string) *zap.Message {
	errBody, _ := json.Marshal(map[string]any{
		"jsonrpc": "2.0",
		"id":      nil,
		"error": map[string]any{
			"code":    -32603,
			"message": errMsg,
		},
	})
	return buildResponse(status, string(errBody))
}
