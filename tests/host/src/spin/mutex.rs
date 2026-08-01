//! カーネルの spin::mutex は画面出力 (Graphic/Printer) に依存しており
//! ホストでは使えないため、sync::queue の import を解決するためだけの
//! 最小限のスタブを置く。
pub struct Mutex<T: ?Sized> {
    _marker: core::marker::PhantomData<T>,
}
