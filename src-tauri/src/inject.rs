use anyhow::Result;
use enigo::{Enigo, Keyboard, Settings};

pub fn type_text(text: &str) -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default())?;
    enigo.text(text)?;
    Ok(())
}
