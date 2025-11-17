#![allow(dead_code)]
// Colours for console messages

pub const RESET: &str = "\x1b[0m";
pub const BLACK: &str = "\x1b[30m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
pub const CYAN: &str = "\x1b[36m";
pub const WHITE: &str = "\x1b[37m";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reset_code() {
        assert_eq!(RESET, "\x1b[0m");
        assert!(!RESET.is_empty());
    }

    #[test]
    fn test_black_code() {
        assert_eq!(BLACK, "\x1b[30m");
        assert!(BLACK.starts_with("\x1b["));
        assert!(BLACK.ends_with("m"));
    }

    #[test]
    fn test_red_code() {
        assert_eq!(RED, "\x1b[31m");
        assert!(RED.contains("31"));
    }

    #[test]
    fn test_green_code() {
        assert_eq!(GREEN, "\x1b[32m");
        assert!(GREEN.contains("32"));
    }

    #[test]
    fn test_yellow_code() {
        assert_eq!(YELLOW, "\x1b[33m");
        assert!(YELLOW.contains("33"));
    }

    #[test]
    fn test_blue_code() {
        assert_eq!(BLUE, "\x1b[34m");
        assert!(BLUE.contains("34"));
    }

    #[test]
    fn test_magenta_code() {
        assert_eq!(MAGENTA, "\x1b[35m");
        assert!(MAGENTA.contains("35"));
    }

    #[test]
    fn test_cyan_code() {
        assert_eq!(CYAN, "\x1b[36m");
        assert!(CYAN.contains("36"));
    }

    #[test]
    fn test_white_code() {
        assert_eq!(WHITE, "\x1b[37m");
        assert!(WHITE.contains("37"));
    }

    #[test]
    fn test_all_colors_have_escape_sequence() {
        let colors = [BLACK, RED, GREEN, YELLOW, BLUE, MAGENTA, CYAN, WHITE, RESET];
        
        for color in &colors {
            assert!(color.starts_with("\x1b["), "Color {} should start with escape sequence", color);
            assert!(color.ends_with("m"), "Color {} should end with 'm'", color);
        }
    }

    #[test]
    fn test_color_codes_are_unique() {
        let colors = vec![
            ("BLACK", BLACK),
            ("RED", RED),
            ("GREEN", GREEN),
            ("YELLOW", YELLOW),
            ("BLUE", BLUE),
            ("MAGENTA", MAGENTA),
            ("CYAN", CYAN),
            ("WHITE", WHITE),
            ("RESET", RESET),
        ];

        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(
                    colors[i].1, colors[j].1,
                    "Colors {} and {} should have different codes",
                    colors[i].0, colors[j].0
                );
            }
        }
    }

    #[test]
    fn test_color_concatenation() {
        let colored_text = format!("{}This is red{}", RED, RESET);
        assert!(colored_text.contains("This is red"));
        assert!(colored_text.starts_with(RED));
        assert!(colored_text.ends_with(RESET));
    }

    #[test]
    fn test_multiple_colors() {
        let multi_color = format!(
            "{}Red {}Green {}Blue {}Reset",
            RED, GREEN, BLUE, RESET
        );
        assert!(multi_color.contains(RED));
        assert!(multi_color.contains(GREEN));
        assert!(multi_color.contains(BLUE));
        assert!(multi_color.contains(RESET));
    }

    #[test]
    fn test_color_length() {
        // All standard color codes should be 5 characters (\x1b[XXm)
        assert_eq!(BLACK.len(), 5);
        assert_eq!(RED.len(), 5);
        assert_eq!(GREEN.len(), 5);
        assert_eq!(YELLOW.len(), 5);
        assert_eq!(BLUE.len(), 5);
        assert_eq!(MAGENTA.len(), 5);
        assert_eq!(CYAN.len(), 5);
        assert_eq!(WHITE.len(), 5);
        assert_eq!(RESET.len(), 4); // Reset is \x1b[0m
    }

    #[test]
    fn test_color_byte_representation() {
        // Test that the escape sequence is correctly represented
        assert_eq!(RED.as_bytes()[0], 0x1b); // ESC character
        assert_eq!(RED.as_bytes()[1], b'[');
        assert_eq!(RED.as_bytes()[2], b'3');
        assert_eq!(RED.as_bytes()[3], b'1');
        assert_eq!(RED.as_bytes()[4], b'm');
    }

    #[test]
    fn test_reset_byte_representation() {
        assert_eq!(RESET.as_bytes()[0], 0x1b);
        assert_eq!(RESET.as_bytes()[1], b'[');
        assert_eq!(RESET.as_bytes()[2], b'0');
        assert_eq!(RESET.as_bytes()[3], b'm');
    }

    #[test]
    fn test_color_in_string_formatting() {
        let message = format!("{}Error:{} Something went wrong", RED, RESET);
        assert!(message.contains("Error:"));
        assert!(message.starts_with(RED));
    }

    #[test]
    fn test_nested_color_usage() {
        let nested = format!(
            "{}Outer{}Inner{}Still Inner{}Back to Outer{}",
            RED, GREEN, YELLOW, GREEN, RESET
        );
        assert!(nested.contains(RED));
        assert!(nested.contains(GREEN));
        assert!(nested.contains(YELLOW));
        assert!(nested.ends_with(RESET));
    }

    #[test]
    fn test_color_constant_immutability() {
        // Ensure constants are static and don't change
        let red1 = RED;
        let red2 = RED;
        assert_eq!(red1, red2);
        assert_eq!(red1.as_ptr(), red2.as_ptr()); // Same memory location
    }

    #[test]
    fn test_all_basic_colors_present() {
        // Verify all 8 basic ANSI colors are defined
        let _basic_colors = [
            BLACK,   // 30
            RED,     // 31
            GREEN,   // 32
            YELLOW,  // 33
            BLUE,    // 34
            MAGENTA, // 35
            CYAN,    // 36
            WHITE,   // 37
        ];
        // If this compiles, all colors are defined
    }

    #[test]
    fn test_color_code_format() {
        // Verify ANSI escape code format: ESC[<code>m
        let verify_format = |color: &str, expected_code: &str| {
            assert!(color.starts_with("\x1b["));
            assert!(color.ends_with("m"));
            assert!(color.contains(expected_code));
        };

        verify_format(BLACK, "30");
        verify_format(RED, "31");
        verify_format(GREEN, "32");
        verify_format(YELLOW, "33");
        verify_format(BLUE, "34");
        verify_format(MAGENTA, "35");
        verify_format(CYAN, "36");
        verify_format(WHITE, "37");
        verify_format(RESET, "0");
    }
}
