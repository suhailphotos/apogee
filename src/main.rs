fn main() {
    // Emit one shell line. Uses $DROPBOX if set, else macOS default path.
    println!(r#"cd "${{DROPBOX:-$HOME/Library/CloudStorage/Dropbox}}""#);
}
