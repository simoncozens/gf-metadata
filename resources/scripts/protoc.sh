
# assumes protoc is installed
# Mac: brew install protoc
# Many Linux: sudo apt-get install protoc

cargo install protobuf-codegen
PATH="$HOME/.cargo/bin:$PATH"

mkdir -p Lib/gfmetadata/

protoc -I resources/protos --rs_out src/ --python_out Lib/gfmetadata resources/protos/*.proto

rm src/mod.rs
