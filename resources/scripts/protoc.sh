
# assumes protoc is installed
# Mac: brew install protoc
# Many Linux: sudo apt-get install protoc

# assumes the following clones:
# ~/oss/gftools is https://github.com/googlefonts/gftools
# ~/oss/fonts is https://github.com/google/fonts

cargo install protobuf-codegen
PATH="$HOME/.cargo/bin:$PATH"

protoc --rs_out gf-metadata/src/ --proto_path ~/oss/gftools/Lib/gftools/ ~/oss/gftools/Lib/gftools/fonts_public.proto

protoc --rs_out gf-metadata/src/ --proto_path ~/oss/fonts/lang/Lib/gflanguages/ ~/oss/fonts/lang/Lib/gflanguages/languages_public.proto