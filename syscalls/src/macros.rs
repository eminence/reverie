#[macro_export]
macro_rules! syscall {
    ($nr:ident)
        => ( $crate::helper::syscall0(
                $crate::nr::$nr) );

    ($nr:ident, $a1:expr)
        => ( $crate::helper::syscall1(
                $crate::nr::$nr,
                $a1 as u64) );

    ($nr:ident, $a1:expr, $a2:expr)
        => ( $crate::helper::syscall2(
                $crate::nr::$nr,
                $a1 as u64, $a2 as u64) );

    ($nr:ident, $a1:expr, $a2:expr, $a3:expr)
        => ( $crate::helper::syscall3(
                $crate::nr::$nr,
                $a1 as u64, $a2 as u64, $a3 as u64) );

    ($nr:ident, $a1:expr, $a2:expr, $a3:expr, $a4:expr)
        => ( $crate::helper::syscall4(
                $crate::nr::$nr,
                $a1 as u64, $a2 as u64, $a3 as u64,
                $a4 as u64) );

    ($nr:ident, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr)
        => ( $crate::helper::syscall5(
                $crate::nr::$nr,
                $a1 as u64, $a2 as u64, $a3 as u64,
                $a4 as u64, $a5 as u64) );

    ($nr:ident, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr)
        => ( $crate::helper::syscall6(
                $crate::nr::$nr,
                $a1 as u64, $a2 as u64, $a3 as u64,
                $a4 as u64, $a5 as u64, $a6 as u64) );
}

