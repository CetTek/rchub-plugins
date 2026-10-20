;; Generated reference proxy: actions remain permission checked by the host.
(module
  (import "rchub" "invoke" (func $invoke (param i32 i32 i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 5 5)
  (func (export "run") (param $entry i32) (param $len i32) (result i32)
    local.get $entry i32.const 0 i32.eq if (result i32)
      i32.const 65536 i32.const 10 i32.const 0 local.get $len
      i32.const 32768 i32.const 262144 call $invoke
    else
    local.get $entry i32.const 1 i32.eq if (result i32)
      i32.const 65546 i32.const 11 i32.const 0 local.get $len
      i32.const 32768 i32.const 262144 call $invoke
    else
    local.get $entry i32.const 2 i32.eq if (result i32)
      i32.const 65557 i32.const 11 i32.const 0 local.get $len
      i32.const 32768 i32.const 262144 call $invoke
    else
      i32.const -1
    end end end
  )
  (data (i32.const 65536) "audio.read")
  (data (i32.const 65546) "audio.write")
  (data (i32.const 65557) "audio.write")
)
