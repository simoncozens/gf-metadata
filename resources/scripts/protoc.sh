
# assumes protoc is installed
# Mac: brew install protoc
# Many Linux: sudo apt-get install protoc

export GFTOOLS="${GFTOOLS:-$HOME/oss/gftools}"

cargo install protobuf-codegen
PATH="$HOME/.cargo/bin:$PATH"

protoc --rs_out src/ --proto_path $GFTOOLS/Lib/gftools $GFTOOLS/Lib/gftools/fonts_public.proto
protoc --rs_out src/ --proto_path $GFTOOLS/Lib/gftools $GFTOOLS/Lib/gftools/designers.proto
protoc --rs_out src/ --proto_path $GFTOOLS/Lib/gftools $GFTOOLS/Lib/gftools/axes.proto

rm src/mod.rs
