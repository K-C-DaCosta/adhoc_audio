function build_recorder() {
    cargo build -p audio_recorder --target=wasm32-unknown-unknown &&
        wasm-bindgen ./target/wasm32-unknown-unknown/debug/audio_recorder.wasm --out-dir ./recorder_output/ --target web
}
function build_recorder_release() {
    cargo build --target=wasm32-unknown-unknown --release &&
        wasm-bindgen ./target/wasm32-unknown-unknown/release/audio_recorder.wasm --out-dir ./recorder_output/ --target web
}

function run_recorder() {
    simple-http-server -p 6969 -- ./recorder_output/
}

case $1 in
build_recorder)
    build_recorder
    ;;
run_recorder)
    run_recorder
    ;;
build_recorder_release)
    build_recorder_release
    ;;
*)
    printf "error!\ncommands are:\nbuild_recorder\n"
    ;;
esac
