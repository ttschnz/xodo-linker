#![windows_subsystem = "windows"]
pub mod linker;
use linker::Linker;

fn main() {
    Linker::read_config("./src/config.yaml").start();
}
