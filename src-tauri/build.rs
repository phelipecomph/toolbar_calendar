fn main() {
    // DPI awareness per-monitor v2 já é aplicado pelo runtime do Tauri (tao).
    // Não sobrescrevemos o manifesto padrão do Tauri para não quebrar o loader.
    tauri_build::build();
}
