use dotenvy::dotenv;
use eyre::{Context, Result};

pub fn load_env() -> Result<()> {
    let path = std::env::current_dir()?;
    println!("Loading environment from path: {}", path.display());
    dotenv().with_context(|| "failed to load .env")?;
    Ok(())
}

/// recursively search for .env file in the current directory and its parents
/// return true if found and loaded, false otherwise
pub fn load_env_recursively() -> Result<bool> {
    let mut path = std::env::current_dir()?;
    println!("Loading environment recursively from path: {}", path.display());
    loop {
        let env_path = path.join(".env");
        if env_path.exists() {
            println!("loading .env from path: {}", env_path.display());
            dotenvy::from_path(&env_path)
                .with_context(|| format!("failed to load .env from path: {}", env_path.display()))?;
            return Ok(true);
        }
        if !path.pop() {
            break;
        }
    }
    Ok(false)
}
