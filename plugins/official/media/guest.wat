;; Generated reference proxy: actions remain permission checked by the host.
(module
  (import "rchub" "invoke" (func $invoke (param i32 i32 i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 2 2)
  (func (export "run") (param $entry i32) (param $len i32) (result i32)
    local.get $entry i32.const 0 i32.eq if (result i32)
      i32.const 8192 i32.const 10 i32.const 0 local.get $len
      i32.const 32768 i32.const 32768 call $invoke
    else
    local.get $entry i32.const 1 i32.eq if (result i32)
      i32.const 8202 i32.const 11 i32.const 0 local.get $len
      i32.const 32768 i32.const 32768 call $invoke
    else
    local.get $entry i32.const 2 i32.eq if (result i32)
      i32.const 8213 i32.const 11 i32.const 0 local.get $len
      i32.const 32768 i32.const 32768 call $invoke
    else
    local.get $entry i32.const 3 i32.eq if (result i32)
      i32.const 8224 i32.const 12 i32.const 0 local.get $len
      i32.const 32768 i32.const 32768 call $invoke
    else
    local.get $entry i32.const 4 i32.eq if (result i32)
      i32.const 8236 i32.const 13 i32.const 0 local.get $len
      i32.const 32768 i32.const 32768 call $invoke
    else
    local.get $entry i32.const 5 i32.eq if (result i32)
      i32.const 8249 i32.const 11 i32.const 0 local.get $len
      i32.const 32768 i32.const 32768 call $invoke
    else
      i32.const -1
    end end end end end end
  )
  (data (i32.const 8192) "audio.read")
  (data (i32.const 8202) "audio.write")
  (data (i32.const 8213) "audio.write")
  (data (i32.const 8224) "display.read")
  (data (i32.const 8236) "display.write")
  (data (i32.const 8249) "media.write")
)
