
# assumes protoc is installed
# Mac: brew install protoc
# Many Linux: sudo apt-get install protoc

cargo install protobuf-codegen
PATH="$HOME/.cargo/bin:$PATH"

protoc --rs_out gf-metadata/src/ --proto_path ~/oss/gftools/Lib/gftools/ ~/oss/gftools/Lib/gftools/fonts_public.proto
