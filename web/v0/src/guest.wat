;; IMPERIUM v0 guest — trampoline with linear memory.
;; Host imports are the only I/O. Network is not imported.
(module
  (import "host" "echo" (func $host_echo (param i32 i32)))
  (import "host" "write" (func $host_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "run_echo") (param $ptr i32) (param $len i32)
    (call $host_echo (local.get $ptr) (local.get $len)))
  (func (export "run_write") (param $pp i32) (param $pl i32) (param $bp i32) (param $bl i32) (result i32)
    (call $host_write (local.get $pp) (local.get $pl) (local.get $bp) (local.get $bl))))
