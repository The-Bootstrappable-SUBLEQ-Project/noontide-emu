pub struct DebugData {
    offsets: Vec<(u64, String)>,
}

pub fn parse_hex_file(inp: &str) -> DebugData {
    let hex_charset: Vec<char> = "0123456789abcdefABCDEF".chars().collect();

    let mut ret: DebugData = DebugData {
        offsets: Vec::new(),
    };

    let mut offset = 0;
    for line in inp.lines() {
        ret.offsets.push((offset, line.to_owned()));
        let mut hex_chars = 0;
        // TODO: Handle hex1/hex2
        for c in line.chars() {
            if c == '#' || c == ';' {
                break;
            }

            if hex_charset.contains(&c) {
                hex_chars += 1;
            }
        }
        assert!(hex_chars % 2 == 0);
        offset += hex_chars / 2;
    }

    ret
}

pub fn memory_dump(mem: &[u8]) -> String {
    let dump_bytes = 0x1000;

    let mut ret = String::new();
    let mut offset = 0;
    while offset < dump_bytes {
        ret.push_str(&format!("{offset:08x}:"));
        for _i in 0..8 {
            let a = mem[offset];
            let b = mem[offset + 1];
            ret.push_str(&format!(" {a:02x}{b:02x}"));
            offset += 2;
        }
        ret.push('\r');
        ret.push('\n');
    }

    ret
}

pub fn render_debug(debug_data: &Option<DebugData>, eip: u64, lines: usize) -> String {
    let Some(debug_data) = debug_data else {
        return "Error: Missing hex0, hex1, or hex2 file for debugging".to_owned();
    };

    let mut ret_lines: Vec<String> = Vec::new();
    let mut cur_line = 0;

    if debug_data.offsets.last().unwrap().0 <= eip {
        return "Error: Current EIP is beyond end of debug file (Run-time generated code?)"
            .to_owned();
    }

    while debug_data.offsets[cur_line].0 <= eip {
        cur_line += 1;
    }
    cur_line -= 1;

    let start = if lines > cur_line {
        0
    } else {
        cur_line - lines
    };

    let end = std::cmp::min(debug_data.offsets.len(), cur_line + lines + 1);
    for i in start..end {
        let line = &debug_data.offsets[i].1;
        if i == cur_line {
            ret_lines.push("->  ".to_owned() + line);
        } else {
            ret_lines.push("    ".to_owned() + line);
        }
    }

    ret_lines.join("\r\n")
}
