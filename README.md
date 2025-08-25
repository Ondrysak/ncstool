# ncstool
Novation Circuit Tracks tool for analyzing and editing NCS session files.

## Features

- **Analyze NCS files**: Parse and display drum patterns, timing, FX settings, and more
- **Clone and Edit**: Create new NCS files with modified drum patterns using a simple text format
- **Pattern Visualization**: ASCII visualization of drum patterns with velocity and probability
- **Comprehensive Parsing**: Support for timing, FX, scenes, chains, and scale settings

## Quick Start

### Build the tool
```bash
cd ncs_tool
cargo build --release
```

### Analyze an NCS file
```bash
./target/release/ncs-tui your_file.ncs
```

### Clone and edit patterns
```bash
# Create a new file with modified drum patterns
./target/release/ncs-tui clone source.ncs target.ncs "0:0:X...X...X...X..."

# Multiple pattern edits
./target/release/ncs-tui clone source.ncs target.ncs \
  "0:0:X...X...X...X..." \
  "1:0:....X.......X..." \
  "2:0:x.x.x.x.x.x.x.x."
```

## Pattern Format

The sequencer format uses: `track:pattern:steps[:probability]`

### Step Characters
- `X` = Strong hit (velocity 127)
- `x` = Weak hit (velocity 32)
- `.` = Rest (velocity 0)
- `0-9` = Specific velocity levels (0, 14, 28, 42, 56, 70, 84, 98, 112, 127)

### Examples
```bash
# Basic kick pattern on track 0, pattern 0
"0:0:X...X...X...X..."

# Hi-hat with probability 7
"1:0:x.x.x.x.x.x.x.x.:7"

# Complex pattern with varying velocities
"0:0:9.5.7.3.9.5.7.3."
```

## Examples

See the `examples/` directory for:
- `pattern_editing_examples.md` - Comprehensive pattern editing guide
- `create_patterns.sh` - Script to generate common drum patterns

## Python CLI (Legacy)

The Python CLI tool provides additional analysis capabilities:

```bash
# Extract drum patterns to JSON
python cli.py drums extract your_file.ncs --json output.json

# Display ASCII patterns
python cli.py drums extract your_file.ncs
```

## File Structure

- `ncs_tool/` - Main Rust tool for analysis and editing
- `cli.py` - Python CLI for additional analysis
- `examples/` - Usage examples and pattern libraries
- `test_data/` - Sample NCS files for testing
- `decompiled_validators/` - Reverse engineering documentation

## Development

### Running Tests
```bash
cd ncs_tool
cargo test
```

### Adding New Patterns
Edit patterns using the simple text format and test with:
```bash
cargo run -- clone test_data/Deep.ncs test_output.ncs "0:0:your_pattern_here"
```

## Ghidra MCP Integration

For reverse engineering work:

Install `uv` and https://github.com/LaurieWired/GhidraMCP

```bash
uv run C:\Users\Ondra\Downloads\ghidra_11.3.2_PUBLIC\Extensions\GhidraMCP-release-1-4\bridge_mcp_ghidra.py --transport sse --mcp-host 127.0.0.1 --mcp-port 8081 --ghidra-server http://127.0.0.1:8080
```

In Augment add: http://127.0.0.1:8081/sse


## IDA Pro MCP 

https://github.com/mrexodia/ida-pro-mcp?tab=readme-ov-file
```bash
uv --directory ./MCPida-pro-mcp run ida-pro-mcp --transport http://127.0.0.1:8744/sse
```