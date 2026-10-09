//! The Signet logo and banner for the terminal (the Python twin is ~/clm-bench/signet_banner.py).
//! The logo is the SVG itself: a 18x18 grid, three small bars on the left and a tall block on the right, #ecebe6 on
//! #0b0b0b. Each terminal cell is one unit wide and two units tall (▀ ▄ █), so the proportions stay those of the SVG.
use std::io::IsTerminal;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const WEBSITE: &str = "signetprotocol.io";
const RECTS: [(i32, i32, i32, i32); 4] = [(0, 0, 5, 4), (0, 7, 5, 4), (0, 14, 5, 4), (8, 0, 10, 18)]; // x, y, w, h
const PAD: i32 = 4;

fn on(x: i32, y: i32) -> bool { RECTS.iter().any(|&(rx, ry, w, h)| rx <= x && x < rx + w && ry <= y && y < ry + h) }

pub fn logo_rows(color: bool) -> Vec<String> {
    let size = 18 + 2 * PAD;
    (0..size / 2).map(|r| {
        let line: String = (0..size).map(|x| match (on(x - PAD, 2 * r - PAD), on(x - PAD, 2 * r + 1 - PAD)) {
            (true, true) => '█', (true, false) => '▀', (false, true) => '▄', _ => ' ',
        }).collect();
        if color { format!("\x1b[38;2;236;235;230m\x1b[48;2;11;11;11m{line}\x1b[0m") } else { line }
    }).collect()
}

/// The logo with the program's name, version and website, side by side.
pub fn print(name: &str) {
    let color = std::io::stdout().is_terminal();
    let (b, d, z) = if color { ("\x1b[1m", "\x1b[2m", "\x1b[0m") } else { ("", "", "") };
    let info = [format!("{b}{name}{z}  v{VERSION}"), format!("{d}{WEBSITE}{z}")];
    let rows = logo_rows(color);
    let top = (rows.len() - info.len()) / 2;
    for (i, row) in rows.iter().enumerate() {
        let extra = i.checked_sub(top).and_then(|j| info.get(j)).map(|s| format!("   {s}")).unwrap_or_default();
        println!("{row}{extra}");
    }
    println!();
}

#[cfg(test)]
mod tests {
    #[test]
    fn same_logo_as_the_svg() {
        let rows = super::logo_rows(false);
        assert_eq!(rows.len(), 13);
        assert_eq!(rows[2].trim_end(), "    █████   ██████████");
        assert_eq!(rows[5].trim_end(), "    ▄▄▄▄▄   ██████████");
        assert_eq!(rows[7].trim_end(), "    ▀▀▀▀▀   ██████████");
    }
}
