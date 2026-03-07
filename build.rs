use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(QmlModule::new("com.howdy.gui").qml_files(["qml/main.qml"]))
        .file("src/bridge.rs")
        .build();
}
