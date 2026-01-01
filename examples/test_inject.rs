use scribe::input::inject::TextInjector;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

/// Example demonstrating text injection via dotool
///
/// This example shows how to use the `TextInjector` to type text into the active window.
/// Run with: `cargo run --example test_inject`
///
/// Requirements:
/// - dotool must be installed and in PATH
/// - You must have focus on a text editor or other text input
///
/// Usage:
/// 1. Run this example
/// 2. When prompted, switch focus to a text editor (5 second delay)
/// 3. Watch the text being typed automatically
fn main() -> anyhow::Result<()> {
    println!("Text Injection Example");
    println!("======================\n");

    // Check if dotool is available
    match TextInjector::new(2) {
        Ok(mut injector) => {
            println!("✓ dotool found\n");

            println!("This example will type text into your active window.");
            println!("\nInstructions:");
            println!("1. Focus on a text editor (you have 5 seconds)");
            println!("2. The example will type several test strings");
            println!("3. Observe the typing behavior\n");

            print!("Press Enter to start...");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            println!("\nSwitching to text editor in:");
            for i in (1..=5).rev() {
                println!("  {i}...");
                thread::sleep(Duration::from_secs(1));
            }

            println!("\nTyping test 1: Simple text");
            injector.inject("Hello from scribe! ")?;
            thread::sleep(Duration::from_secs(1));

            println!("Typing test 2: Text with punctuation");
            injector.inject("This is a test with punctuation: hello, world! ")?;
            thread::sleep(Duration::from_secs(1));

            println!("Typing test 3: Numbers and symbols");
            injector.inject("123 + 456 = 579. Special chars: @#$%^&*() ")?;
            thread::sleep(Duration::from_secs(1));

            println!("Typing test 4: Multiple sentences");
            injector.inject("First sentence. Second sentence. Third sentence. ")?;
            thread::sleep(Duration::from_secs(1));

            println!("\n✓ All tests completed!");
            println!(
                "\nNote: If you see the text in your editor, text injection is working correctly."
            );
        }
        Err(e) => {
            eprintln!("✗ Error: {e}");
            eprintln!("\nTo fix this:");
            eprintln!("  1. Install dotool: cargo install dotool");
            eprintln!("  2. Or on Arch: paru -S dotool");
            eprintln!("  3. Ensure dotool is in your PATH");
            std::process::exit(1);
        }
    }

    Ok(())
}
