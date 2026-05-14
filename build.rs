fn main() {
    embuild::build::CfgArgs::output_propagated("ESP_IDF_HAL").unwrap();
    embuild::build::LinkArgs::output_propagated("ESP_IDF_HAL").unwrap();
}
