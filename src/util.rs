pub(crate) fn measure<F, T>(label: &str, func: F) -> T
where
    F: FnOnce() -> T,
{
    let instant = std::time::Instant::now();
    let value = func();
    log::debug!("{} took {:?}", label, instant.elapsed());
    value
}
