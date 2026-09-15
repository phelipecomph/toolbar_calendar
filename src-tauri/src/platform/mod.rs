// Toda dependência Win32 vive aqui. Resto do código não sabe que Windows existe.
#[cfg(windows)]
pub mod appbar;
