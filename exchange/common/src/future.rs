#[macro_export]
macro_rules! await_or_insert_with {
    ($opt: expr, $init: expr) => {{
        let task = $opt.get_or_insert_with($init);
        let result = task.await;
        $opt = None;
        result
    }};
}
