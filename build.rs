use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=qml/");
    println!("cargo:rerun-if-changed=build.rs");
    CxxQtBuilder::new_qml_module(QmlModule::new("com.howdy.gui").qml_files(["qml/main.qml"]))
        .file("src/bridge.rs")
        .build();
}
