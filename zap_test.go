// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// See the file LICENSE for licensing terms.

package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
	"time"

	"github.com/luxfi/zap"
)

// TestStartZAP verifies the ZAP node starts, accepts connections, and proxies
// requests to a mock EVM RPC server.
func TestStartZAP(t *testing.T) {
	// Mock EVM RPC server.
	rpcServer := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, _ := io.ReadAll(r.Body)
		var req struct {
			Method string          `json:"method"`
			Params json.RawMessage `json:"params"`
		}
		json.Unmarshal(body, &req)

		resp := map[string]any{
			"jsonrpc": "2.0",
			"id":      1,
			"result": map[string]any{
				"method": req.Method,
				"ok":     true,
			},
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(resp)
	}))
	defer rpcServer.Close()

	// Find a free port for ZAP.
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	port := listener.Addr().(*net.TCPAddr).Port
	listener.Close()

	logger := slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelDebug}))

	zapNode, err := startZAP(port, rpcServer.URL, logger)
	if err != nil {
		t.Fatal("startZAP failed:", err)
	}
	defer zapNode.Stop()

	// Give the listener a moment to be ready.
	time.Sleep(50 * time.Millisecond)

	// Connect as a ZAP client.
	conn, err := net.DialTimeout("tcp", fmt.Sprintf("127.0.0.1:%d", port), 2*time.Second)
	if err != nil {
		t.Fatal("dial failed:", err)
	}
	defer conn.Close()

	// Send handshake (required by ZAP node protocol).
	sendHandshake(t, conn, "test-client-1")
	readHandshake(t, conn)

	// Send a MsgOrderbook request.
	sendZAPRequest(t, conn, MsgOrderbook, "dex.GetOrderbook", `[{"symbol":"ZOO/USDL","depth":10}]`)

	// Read response.
	respMsg := readZAPMessage(t, conn)
	root := respMsg.Root()
	status := root.Uint32(fieldStatus)
	body := root.Text(fieldBody)

	if status != 200 {
		t.Fatalf("expected status 200, got %d", status)
	}

	var rpcResp struct {
		Result struct {
			Method string `json:"method"`
			OK     bool   `json:"ok"`
		} `json:"result"`
	}
	if err := json.Unmarshal([]byte(body), &rpcResp); err != nil {
		t.Fatalf("failed to unmarshal response body: %v\nbody: %s", err, body)
	}
	if rpcResp.Result.Method != "dex.GetOrderbook" {
		t.Fatalf("expected method dex.GetOrderbook, got %s", rpcResp.Result.Method)
	}
	if !rpcResp.Result.OK {
		t.Fatal("expected ok=true")
	}
}

// TestStartZAPPortConflict verifies that starting ZAP on an occupied port fails.
func TestStartZAPPortConflict(t *testing.T) {
	// Listen on all interfaces (same as ZAP node does) to block the port.
	listener, err := net.Listen("tcp", ":0")
	if err != nil {
		t.Fatal(err)
	}
	port := listener.Addr().(*net.TCPAddr).Port
	// Don't close — keep it occupied.
	defer listener.Close()

	logger := slog.New(slog.NewTextHandler(io.Discard, nil))
	_, err = startZAP(port, "http://127.0.0.1:1", logger)
	if err == nil {
		t.Fatal("expected error when port is occupied")
	}
}

// TestZapPortFromEnv verifies the env var parsing.
func TestZapPortFromEnv(t *testing.T) {
	// Default
	os.Unsetenv("ZAP_PORT")
	if got := zapPortFromEnv(); got != defaultZAPPort {
		t.Fatalf("expected %d, got %d", defaultZAPPort, got)
	}

	// Custom
	os.Setenv("ZAP_PORT", "12345")
	defer os.Unsetenv("ZAP_PORT")
	if got := zapPortFromEnv(); got != 12345 {
		t.Fatalf("expected 12345, got %d", got)
	}

	// Invalid
	os.Setenv("ZAP_PORT", "not-a-number")
	if got := zapPortFromEnv(); got != defaultZAPPort {
		t.Fatalf("expected %d for invalid input, got %d", defaultZAPPort, got)
	}
}

// TestAllHandlers verifies that MsgQuote, MsgSwap, and MsgOrderbook all route correctly.
func TestAllHandlers(t *testing.T) {
	var receivedMethod string
	rpcServer := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, _ := io.ReadAll(r.Body)
		var req struct {
			Method string `json:"method"`
		}
		json.Unmarshal(body, &req)
		receivedMethod = req.Method

		resp := map[string]any{"jsonrpc": "2.0", "id": 1, "result": "ok"}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(resp)
	}))
	defer rpcServer.Close()

	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	port := listener.Addr().(*net.TCPAddr).Port
	listener.Close()

	logger := slog.New(slog.NewTextHandler(io.Discard, nil))
	zapNode, err := startZAP(port, rpcServer.URL, logger)
	if err != nil {
		t.Fatal(err)
	}
	defer zapNode.Stop()

	time.Sleep(50 * time.Millisecond)

	tests := []struct {
		msgType        uint16
		expectedMethod string
	}{
		{MsgQuote, "dex.GetQuote"},
		{MsgSwap, "dex.Swap"},
		{MsgOrderbook, "dex.GetOrderbook"},
	}

	for _, tt := range tests {
		t.Run(tt.expectedMethod, func(t *testing.T) {
			conn, err := net.DialTimeout("tcp", fmt.Sprintf("127.0.0.1:%d", port), 2*time.Second)
			if err != nil {
				t.Fatal(err)
			}
			defer conn.Close()

			sendHandshake(t, conn, fmt.Sprintf("client-%d", tt.msgType))
			readHandshake(t, conn)

			// Send request with empty method — handler should use default.
			sendZAPRequest(t, conn, tt.msgType, "", `[]`)
			resp := readZAPMessage(t, conn)

			if resp.Root().Uint32(fieldStatus) != 200 {
				t.Fatalf("expected 200, got %d", resp.Root().Uint32(fieldStatus))
			}

			if receivedMethod != tt.expectedMethod {
				t.Fatalf("expected method %s, got %s", tt.expectedMethod, receivedMethod)
			}
		})
	}
}

// TestMethodOverride verifies that a method in the ZAP message overrides the default.
func TestMethodOverride(t *testing.T) {
	var receivedMethod string
	rpcServer := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, _ := io.ReadAll(r.Body)
		var req struct {
			Method string `json:"method"`
		}
		json.Unmarshal(body, &req)
		receivedMethod = req.Method

		resp := map[string]any{"jsonrpc": "2.0", "id": 1, "result": "ok"}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(resp)
	}))
	defer rpcServer.Close()

	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	port := listener.Addr().(*net.TCPAddr).Port
	listener.Close()

	logger := slog.New(slog.NewTextHandler(io.Discard, nil))
	zapNode, err := startZAP(port, rpcServer.URL, logger)
	if err != nil {
		t.Fatal(err)
	}
	defer zapNode.Stop()

	time.Sleep(50 * time.Millisecond)

	conn, err := net.DialTimeout("tcp", fmt.Sprintf("127.0.0.1:%d", port), 2*time.Second)
	if err != nil {
		t.Fatal(err)
	}
	defer conn.Close()

	sendHandshake(t, conn, "override-client")
	readHandshake(t, conn)

	// Send MsgQuote but with a custom method override.
	sendZAPRequest(t, conn, MsgQuote, "eth_blockNumber", `[]`)
	resp := readZAPMessage(t, conn)

	if resp.Root().Uint32(fieldStatus) != 200 {
		t.Fatalf("expected 200, got %d", resp.Root().Uint32(fieldStatus))
	}
	if receivedMethod != "eth_blockNumber" {
		t.Fatalf("expected eth_blockNumber, got %s", receivedMethod)
	}
}

// --- helpers ---

func sendHandshake(t *testing.T, conn net.Conn, nodeID string) {
	t.Helper()
	b := zap.NewBuilder(128)
	obj := b.StartObject(64)
	idBytes := []byte(nodeID)
	for i, c := range idBytes {
		if i >= 60 {
			break
		}
		obj.SetUint8(i, c)
	}
	obj.SetUint32(60, uint32(len(idBytes)))
	obj.FinishAsRoot()
	data := b.Finish()
	writeWireMessage(t, conn, data)
}

func readHandshake(t *testing.T, conn net.Conn) {
	t.Helper()
	conn.SetReadDeadline(time.Now().Add(5 * time.Second))
	readWireMessage(t, conn)
	conn.SetReadDeadline(time.Time{})
}

func sendZAPRequest(t *testing.T, conn net.Conn, msgType uint16, method, params string) {
	t.Helper()

	b := zap.NewBuilder(256)
	obj := b.StartObject(fieldReqSize)
	if method != "" {
		obj.SetText(fieldMethod, method)
	}
	if params != "" {
		obj.SetText(fieldParams, params)
	}
	obj.FinishAsRoot()

	// Set flags with message type in upper 8 bits.
	flags := msgType << 8
	data := b.FinishWithFlags(flags)

	// Wrap as a Call request (8-byte header: reqID + reqFlag).
	wrapped := make([]byte, len(data)+8)
	// reqID = 1
	wrapped[0] = 1
	wrapped[1] = 0
	wrapped[2] = 0
	wrapped[3] = 0
	// reqFlag = ReqFlagReq (1)
	wrapped[4] = 1
	wrapped[5] = 0
	wrapped[6] = 0
	wrapped[7] = 0
	copy(wrapped[8:], data)

	writeWireMessage(t, conn, wrapped)
}

func readZAPMessage(t *testing.T, conn net.Conn) *zap.Message {
	t.Helper()
	conn.SetReadDeadline(time.Now().Add(5 * time.Second))
	defer conn.SetReadDeadline(time.Time{})

	data := readWireMessage(t, conn)

	// Skip the 8-byte Call correlation header if present.
	if len(data) >= 8 {
		// Check if bytes 4-8 are ReqFlagResp (2).
		flag := uint32(data[4]) | uint32(data[5])<<8 | uint32(data[6])<<16 | uint32(data[7])<<24
		if flag == 2 { // ReqFlagResp
			data = data[8:]
		}
	}

	msg, err := zap.Parse(data)
	if err != nil {
		t.Fatalf("failed to parse ZAP response: %v (data len=%d)", err, len(data))
	}
	return msg
}

// writeWireMessage writes a length-prefixed message to the connection.
func writeWireMessage(t *testing.T, conn net.Conn, data []byte) {
	t.Helper()
	lenBuf := make([]byte, 4)
	lenBuf[0] = byte(len(data))
	lenBuf[1] = byte(len(data) >> 8)
	lenBuf[2] = byte(len(data) >> 16)
	lenBuf[3] = byte(len(data) >> 24)
	if _, err := conn.Write(lenBuf); err != nil {
		t.Fatal("write len:", err)
	}
	if _, err := conn.Write(data); err != nil {
		t.Fatal("write data:", err)
	}
}

// readWireMessage reads a length-prefixed message from the connection.
func readWireMessage(t *testing.T, conn net.Conn) []byte {
	t.Helper()
	lenBuf := make([]byte, 4)
	if _, err := io.ReadFull(conn, lenBuf); err != nil {
		t.Fatal("read len:", err)
	}
	length := uint32(lenBuf[0]) | uint32(lenBuf[1])<<8 | uint32(lenBuf[2])<<16 | uint32(lenBuf[3])<<24
	if length > 10*1024*1024 {
		t.Fatalf("message too large: %d", length)
	}
	data := make([]byte, length)
	if _, err := io.ReadFull(conn, data); err != nil {
		t.Fatal("read data:", err)
	}
	return data
}

// Ensure context import is used (for handler signature).
var _ = context.Background
