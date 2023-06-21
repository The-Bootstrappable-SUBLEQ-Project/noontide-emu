use std::collections::HashMap;

pub struct DebugData {
    pub offsets: Vec<(u64, String)>,
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
        let mut wait_for_space = false;
        for c in line.chars() {
            // Comments
            if c == '#' || c == ';' {
                break;
            }

            if c == ' ' {
                wait_for_space = false;
            }

            if wait_for_space {
                continue;
            }

            if c == '?' || c == '&' || c == ':' {
                if c != ':' {
                    hex_chars += 16;
                }

                wait_for_space = true;
                continue;
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

fn inc_ref_count(ref_counts: &mut HashMap<String, u64>, token: &str) {
    *ref_counts.entry(token.to_owned()).or_insert(0) += 1;
}

pub fn parse_lsq_file(inp: &str) -> DebugData {
    let mut ret: DebugData = DebugData {
        offsets: Vec::new(),
    };

    let mut ref_counts: HashMap<String, u64> = HashMap::new();
    for line in inp.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if !tokens.is_empty() {
            let inst = tokens[0];
            match inst {
                "abssq" | "relsq" | "lblsq" => {
                    inc_ref_count(&mut ref_counts, tokens[1]);
                    inc_ref_count(&mut ref_counts, tokens[2]);
                    if inst == "lblsq" {
                        inc_ref_count(&mut ref_counts, tokens[3]);
                    }
                }
                "subaddr" => {
                    inc_ref_count(&mut ref_counts, tokens[2]);
                }
                "raw_ref" => {
                    for token in &tokens[1..] {
                        inc_ref_count(&mut ref_counts, token.to_owned());
                    }
                }
                _ => {}
            }
        }
    }

    let mut offset: u64 = 0;
    for line in inp.lines() {
        ret.offsets.push((offset, line.to_owned()));

        let tokens: Vec<&str> = line.split_whitespace().collect();
        if !tokens.is_empty() {
            let inst = tokens[0];
            match inst {
                "abssq" | "relsq" | "lblsq" => {
                    offset += 24;
                }
                "raw" | "raw_ref" => {
                    offset += TryInto::<u64>::try_into(8 * (tokens.len() - 1)).unwrap();
                }
                "subaddr" | "zeroaddr" => {
                    offset += 24 * ref_counts.get(tokens[1]).unwrap();
                }

                _ => {}
            }
        }
    }

    eprintln!("Code size: {0} ({0:#x}) bytes", offset);
    ret
}

pub fn find_debug_data(base_path: &str) -> Option<DebugData> {
    let mut debug_data: Option<DebugData> = None;
    for ext in ["hex0", "hex1", "hex2"] {
        let mut hex_path_str = base_path.to_owned();
        hex_path_str.push('.');
        hex_path_str.push_str(ext);
        let hex_path = std::path::Path::new(&hex_path_str);
        if !hex_path.exists() {
            continue;
        }

        debug_data = Some(parse_hex_file(&std::fs::read_to_string(hex_path).unwrap()));
    }

    let mut lsq_path_str = base_path.to_owned();
    lsq_path_str.push_str(".lsq");
    let lsq_path = std::path::Path::new(&lsq_path_str);
    if lsq_path.exists() {
        debug_data = Some(parse_lsq_file(&std::fs::read_to_string(lsq_path).unwrap()));
    }

    debug_data
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

pub fn render_debug(
    debug_data: &Option<DebugData>,
    eip: u64,
    lines: usize,
    batch: bool,
) -> (isize, String) {
    let Some(debug_data) = debug_data else {
        return (-1, "Error: Missing hex0, hex1, hex2, or lsq file for debugging".to_owned());
    };

    let mut ret_lines: Vec<String> = Vec::new();
    let mut cur_line = 0;

    if debug_data.offsets.last().unwrap().0 <= eip {
        return (
            -1,
            "Error: Current EIP is beyond end of debug file (Run-time generated code?)".to_owned(),
        );
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

    if batch {
        (cur_line.try_into().unwrap(), ret_lines.join("\n"))
    } else {
        (cur_line.try_into().unwrap(), ret_lines.join("\r\n"))
    }
}
