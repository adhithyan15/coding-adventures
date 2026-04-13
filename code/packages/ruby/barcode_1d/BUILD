if [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ]; then (cd ../paint_vm_metal_native && bash BUILD) && (cd ../paint_codec_png_native && bash BUILD); fi
ruby -Ilib -I../code39/lib -I../barcode_layout_1d/lib -I../paint_instructions/lib -I../pixel_container/lib -I../paint_vm_metal_native/lib -I../paint_codec_png_native/lib test/test_barcode_1d.rb
