# ncstool
Novation Circuit Tracks tool


# ghidra MCP 
install `uv`

install https://github.com/LaurieWired/GhidraMCP


```bash
uv run C:\Users\Ondra\Downloads\ghidra_11.3.2_PUBLIC\Extensions\GhidraMCP-release-1-4\bridge_mcp_ghidra.py --transport sse --mcp-host 127.0.0.1 --mcp-port 8081 --ghidra-server http://127.0.0.1:8080
```

in augment add http://127.0.0.1:8081/sse