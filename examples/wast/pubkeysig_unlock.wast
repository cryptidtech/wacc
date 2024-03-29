(module
  ;; importing the wacc push functions
  (import "wacc" "_push" (func $push (param i32 i32) (result i32)))

  ;; function to provide a solution for a pubkey signature lock
  (func $main (export "for_great_justice") (param) (result i32)
    ;; "/entry/"
    i32.const 0
    i32.const 7
    call $push

    ;; "/entry/proof"
    i32.const 7
    i32.const 12
    call $push

    return
  )

  ;; export the memory
  (memory (export "memory") 1)

  ;; String constants for referenceing key-value pairs
  ;;
  ;;                   [NAME]                 [IDX] [LEN]
  (data (i32.const  0) "/entry/"       )  ;;     0     7
  (data (i32.const  7) "/entry/proof"  )  ;;     7     12
)
