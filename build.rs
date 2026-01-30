use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new()
        .qml_module(QmlModule {
            uri: "com.howdy.gui",
            rust_files: &["src/bridge.rs"],
            qml_files: &["qml/main.qml"],
            ..Default::default()
        })
        .build();
}
