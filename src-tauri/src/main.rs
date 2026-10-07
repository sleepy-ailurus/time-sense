// Windows 下固定使用 GUI 子系统：不管 debug 还是 release，都不会出现控制台黑框。
// 之前是 cfg_attr(not(debug_assertions), ...)，也就是 dev/debug 运行时会走默认的 CONSOLE 子系统，
// 从任务栏 Jump List、资源管理器启动就会弹出一个控制台窗口。
// 从终端（cmd / PowerShell / IDE 终端）启动时，标准输出句柄由父进程继承，tracing 日志照常打印。
#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    timesense_lib::run();
}
