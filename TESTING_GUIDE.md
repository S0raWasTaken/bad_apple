# Testing Guide

This document describes the comprehensive unit test suite added to the project.

## Overview

A total of **97 unit tests** have been added across 4 source files, covering all functionality modified in the current branch.

## Test Files

### 1. asciic/src/children.rs (7 tests)

**Module**: FFmpeg command execution wrapper

**Tests**:
- Binary existence and error handling
- Argument passing and validation
- Exit code handling
- Special characters in arguments
- Path verification

**Key test cases**:
```rust
test_ffmpeg_with_nonexistent_binary()
test_ffmpeg_with_invalid_args()
test_ffmpeg_with_special_characters_in_args()
```

### 2. asciic/src/cli.rs (21 tests)

**Module**: Command-line argument parsing using clap

**Tests**:
- Style enum ANSI code mapping
- Input/output path handling
- Default value behavior
- Extension management (.bapple, .txt)
- Flag validation (colorize, no_audio, threshold)

**Key test cases**:
```rust
test_args_handle_io_with_video()
test_args_handle_io_image_default_output()
test_style_ansi_fgpaint()
test_args_with_paths_containing_special_chars()
```

### 3. asciic/src/colours.rs (21 tests)

**Module**: ANSI color code constants

**Tests**:
- Individual color code validation
- Escape sequence format compliance
- Color uniqueness verification
- Byte representation accuracy
- String concatenation behavior

**Key test cases**:
```rust
test_all_colors_have_escape_sequence()
test_color_codes_are_unique()
test_color_byte_representation()
```

### 4. asciic/src/primitives.rs (48 tests)

**Module**: Core data structures and algorithms

**Tests**:
- Input enum (Video/Image variants)
- Metadata struct (fps, frametime, version)
- Charset (brightness-to-character mapping)
- Color difference calculation
- Tar archive operations
- Edge cases and integration scenarios

**Key test cases**:
```rust
test_charset_match_char_all_ranges()
test_get_max_colour_diff_black_to_white()
test_metadata_version_format()
test_charset_covers_full_brightness_range()
```

## Running Tests

### Run all tests
```bash
cd asciic
cargo test
```

### Run tests for a specific module
```bash
cargo test --test children
cargo test --test cli
cargo test --test colours
cargo test --test primitives
```

### Run a specific test
```bash
cargo test test_ffmpeg_with_nonexistent_binary
```

### Run with output
```bash
cargo test -- --nocapture
```

### Run tests in parallel (default) or serial
```bash
cargo test -- --test-threads=1  # Serial execution
```

## Test Organization

All tests follow Rust best practices:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_descriptive_name() {
        // Arrange
        let input = create_test_input();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

## Test Categories

### Unit Tests
Direct testing of individual functions and methods with controlled inputs.

### Integration Tests
Testing interactions between components (e.g., `test_args_handle_io_with_video`).

### Property Tests
Testing invariants across ranges of inputs (e.g., `test_charset_covers_full_brightness_range`).

### Edge Case Tests
Boundary conditions, zero values, extremes (e.g., `test_get_max_colour_diff_black_to_white`).

### Error Path Tests
Validating error handling (e.g., `test_ffmpeg_with_invalid_args`).

## Platform-Specific Tests

Some tests are platform-specific and use conditional compilation:

```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    // Unix-specific test code
}
```

These tests will only run on Unix-like systems (Linux, macOS).

## Test Coverage Summary

| Component | Coverage |
|-----------|----------|
| FFmpeg execution | ✅ Complete |
| CLI parsing | ✅ Complete |
| ANSI colors | ✅ Complete |
| Input types | ✅ Complete |
| Metadata | ✅ Complete |
| Charset | ✅ Complete |
| Color diff | ✅ Complete |
| Archive ops | ✅ Complete |

## Adding New Tests

When adding new functionality:

1. Add tests in the same file within the `#[cfg(test)] mod tests` block
2. Use descriptive names: `test_<function>_<scenario>_<expected>`
3. Follow the Arrange-Act-Assert pattern
4. Test both success and failure cases
5. Consider edge cases and boundary conditions

Example:
```rust
#[test]
fn test_new_function_with_valid_input() {
    let input = valid_test_input();
    let result = new_function(input);
    assert!(result.is_ok());
}

#[test]
fn test_new_function_with_invalid_input() {
    let input = invalid_test_input();
    let result = new_function(input);
    assert!(result.is_err());
}
```

## Troubleshooting

### Test fails with "no such file or directory"
- Check that test files use temporary directories (TempDir)
- Verify paths are relative to the test execution directory

### Test fails on Windows but passes on Unix
- Check for platform-specific code
- Use `#[cfg(unix)]` or `#[cfg(windows)]` as needed

### Flaky tests
- Ensure tests don't depend on external state
- Use controlled test data, not random values
- Avoid timing-dependent assertions

## Continuous Integration

Tests should be run automatically on:
- Every commit (pre-commit hook)
- Pull requests (CI pipeline)
- Before releases

Example GitHub Actions:
```yaml
- name: Run tests
  run: cargo test --all-features
```

## Test Maintenance

- Update tests when functionality changes
- Remove obsolete tests
- Keep test data fixtures current
- Review test coverage regularly

## Further Reading

- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Cargo Test Documentation](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
- [Writing Tests in Rust](https://doc.rust-lang.org/rust-by-example/testing.html)