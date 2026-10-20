;; Generated reference proxy: actions remain permission checked by the host.
(module
  (import "rchub" "invoke" (func $invoke (param i32 i32 i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 2 2)
  (func (export "run") (param $entry i32) (param $len i32) (result i32)
    local.get $entry i32.const 0 i32.eq if (result i32)
      i32.const 8192 i32.const 11 i32.const 0 local.get $len
      i32.const 32768 i32.const 32768 call $invoke
    else
      i32.const -1
    end
  )
  (data (i32.const 8192) "system.read")
)
