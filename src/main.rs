// FreeBSD speaker device network server. CLI entry point that selects an
// output backend, initialises logging (stderr or syslog), optionally
// daemonises, and starts the HTTP server. Two backends exist: the FreeBSD
// /dev/speaker writer (always compiled), and a CPAL audio renderer (gated
// behind the `cpal` Cargo feature, on by default). The --output flag chooses
// the backend; in `auto` mode the device path is probed and CPAL is used as
// fallback when available. Without the `cpal` feature, `auto` falls through
// to freebsd-speaker and fails at startup if the device path is missing.
// Backend-specific flags from the unselected backend are warned about, not
// rejected.

use clap::{Parser, ValueEnum};
use daemonize::Daemonize;
use log::{error, info};
#[cfg(feature = "cpal")]
use log::warn;
use std::process;
#[cfg(feature = "cpal")]
use std::sync::Arc;
use std::time::Duration;
use syslog::{BasicLogger, Facility, Formatter3164};

#[cfg(feature = "cpal")]
use spkrd::cpal_backend::{CpalBackend, CpalConfig, Waveform};
use spkrd::server::{self, Backend};

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum OutputMode {
    Auto,
    FreebsdSpeaker,
    #[cfg(feature = "cpal")]
    Cpal,
}

#[cfg(feature = "cpal")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum WaveformArg {
    Square,
    SquareBandlimited,
    Sine,
    Triangle,
    Sawtooth,
    PcSpeaker,
}

#[cfg(feature = "cpal")]
impl From<WaveformArg> for Waveform {
    fn from(w: WaveformArg) -> Self {
        match w {
            WaveformArg::Square => Waveform::Square,
            WaveformArg::SquareBandlimited => Waveform::SquareBandlimited,
            WaveformArg::Sine => Waveform::Sine,
            WaveformArg::Triangle => Waveform::Triangle,
            WaveformArg::Sawtooth => Waveform::Sawtooth,
            WaveformArg::PcSpeaker => Waveform::PcSpeaker,
        }
    }
}

#[cfg(feature = "cpal")]
const OUTPUT_HELP: &str =
    "Output backend: auto picks freebsd-speaker if --device exists, else cpal";
#[cfg(not(feature = "cpal"))]
const OUTPUT_HELP: &str =
    "Output backend: auto requires --device to exist (CPAL fallback not compiled in)";

#[derive(Parser)]
#[command(author, version, about = "FreeBSD speaker device network server", long_about = None)]
struct Args {
    #[arg(short, long, default_value = "1111")]
    port: u16,

    #[arg(short, long, default_value = "30", help = "Retry timeout in seconds")]
    retry_timeout: u64,

    #[arg(
        long,
        default_value_t = 1000,
        help = "Maximum melody body length in bytes (1..=1048576)"
    )]
    max_melody_length: usize,

    #[arg(
        long,
        value_enum,
        default_value_t = OutputMode::Auto,
        help = OUTPUT_HELP,
    )]
    output: OutputMode,

    #[arg(
        short,
        long,
        default_value = "/dev/speaker",
        help = "Path to speaker device (used by freebsd-speaker backend)"
    )]
    device: String,

    #[arg(long, help = "Run as daemon in background")]
    daemon: bool,

    #[arg(
        long,
        default_value = "/var/run/spkrd.pid",
        help = "Path to PID file (requires appropriate permissions for default path)"
    )]
    pidfile: String,

    #[arg(short = 'D', long, help = "Enable debug logging including client request details")]
    debug: bool,

    // CPAL-only options (only present when built with the `cpal` feature).
    #[cfg(feature = "cpal")]
    #[arg(
        long,
        value_enum,
        default_value_t = WaveformArg::PcSpeaker,
        help = "[cpal] waveform"
    )]
    waveform: WaveformArg,

    #[cfg(feature = "cpal")]
    #[arg(long, default_value_t = 0.25, help = "[cpal] output volume in [0.0, 1.0]")]
    volume: f32,

    #[cfg(feature = "cpal")]
    #[arg(long, help = "[cpal] sample rate in Hz; falls back to device default")]
    sample_rate: Option<u32>,

    #[cfg(feature = "cpal")]
    #[arg(
        long,
        help = "[cpal] audio host: ALSA (default), PipeWire (requires --features pipewire), \
                PulseAudio (requires --features pulseaudio), JACK (requires --features jack). \
                Matching is case-insensitive. When omitted, cpal picks the best available host \
                automatically (PipeWire > PulseAudio > ALSA)."
    )]
    cpal_host: Option<String>,

    #[cfg(feature = "cpal")]
    #[arg(long, help = "[cpal] output device name; defaults to system default")]
    cpal_device: Option<String>,
}

fn init_logging(daemon: bool, debug: bool) {
    if daemon {
        let formatter = Formatter3164 {
            facility: Facility::LOG_DAEMON,
            hostname: None,
            process: "spkrd".into(),
            pid: process::id(),
        };
        match syslog::unix(formatter) {
            Ok(logger) => {
                log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
                    .map(|()| {
                        log::set_max_level(if debug {
                            log::LevelFilter::Debug
                        } else {
                            log::LevelFilter::Info
                        })
                    })
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to initialize syslog: {}", e);
                        process::exit(1);
                    });
            }
            Err(e) => {
                eprintln!("Failed to connect to syslog: {}", e);
                process::exit(1);
            }
        }
    } else {
        env_logger::Builder::new()
            .filter_level(if debug {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Info
            })
            .init();
    }
}

// Resolve `auto` to a concrete mode by checking whether `device` exists.
// With the `cpal` feature compiled in, a missing device falls back to CPAL.
// Without it, `auto` always resolves to FreebsdSpeaker; main() then verifies
// the device exists and fails at startup if it does not.
fn resolve_output(mode: OutputMode, device: &str) -> OutputMode {
    match mode {
        OutputMode::Auto => {
            let exists = std::fs::metadata(device).is_ok();
            #[cfg(feature = "cpal")]
            {
                if exists {
                    OutputMode::FreebsdSpeaker
                } else {
                    OutputMode::Cpal
                }
            }
            #[cfg(not(feature = "cpal"))]
            {
                let _ = exists;
                OutputMode::FreebsdSpeaker
            }
        }
        other => other,
    }
}

fn warn_unused_flags(args: &Args, resolved: OutputMode, user_specified_output: bool) {
    #[cfg(feature = "cpal")]
    let cpal_specific_set = args.waveform != WaveformArg::PcSpeaker
        || args.volume != 0.25
        || args.sample_rate.is_some()
        || args.cpal_host.is_some()
        || args.cpal_device.is_some();
    let device_specific_set = args.device != "/dev/speaker";

    match resolved {
        OutputMode::FreebsdSpeaker => {
            #[cfg(feature = "cpal")]
            if cpal_specific_set {
                warn!(
                    "CPAL-specific flags (--waveform/--volume/--sample-rate/--cpal-host/--cpal-device) are ignored under --output=freebsd-speaker"
                );
            }
        }
        #[cfg(feature = "cpal")]
        OutputMode::Cpal => {
            if device_specific_set && user_specified_output {
                warn!("--device is ignored under --output=cpal");
            }
        }
        OutputMode::Auto => {}
    }
    let _ = (user_specified_output, device_specific_set);
}

fn build_backend(args: &Args, resolved: OutputMode) -> Result<Backend, Box<dyn std::error::Error>> {
    match resolved {
        OutputMode::FreebsdSpeaker => Ok(Backend::FreebsdSpeaker {
            device_path: args.device.clone(),
        }),
        #[cfg(feature = "cpal")]
        OutputMode::Cpal => {
            let cfg = CpalConfig {
                host: args.cpal_host.clone(),
                device: args.cpal_device.clone(),
                sample_rate: args.sample_rate,
                volume: args.volume.clamp(0.0, 1.0),
                waveform: args.waveform.into(),
            };
            let backend = CpalBackend::new(&cfg)?;
            Ok(Backend::Cpal(Arc::new(backend)))
        }
        OutputMode::Auto => unreachable!("auto should be resolved before build_backend"),
    }
}

// Hard ceiling on the melody length limit. The body is held in memory before
// validation, so an operator-supplied limit above this is rejected at startup
// to avoid plausible-misconfiguration OOMs.
const MAX_MELODY_LENGTH_CEILING: usize = 1024 * 1024;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let retry_timeout = Duration::from_secs(args.retry_timeout);

    if args.max_melody_length == 0 {
        eprintln!("spkrd: --max-melody-length must be at least 1");
        process::exit(1);
    }
    if args.max_melody_length > MAX_MELODY_LENGTH_CEILING {
        eprintln!(
            "spkrd: --max-melody-length {} exceeds ceiling of {} bytes (1 MiB)",
            args.max_melody_length, MAX_MELODY_LENGTH_CEILING
        );
        process::exit(1);
    }

    init_logging(args.daemon, args.debug);

    // Track whether user explicitly chose --output (vs Auto default) for the
    // purposes of "ignored flag" warnings.
    let user_specified_output = args.output != OutputMode::Auto;

    // Without the `cpal` feature there is no fallback backend, so a missing
    // device path under --output=auto is a startup error rather than the
    // silent fall-through that the freebsd-speaker backend would otherwise
    // produce on each request.
    #[cfg(not(feature = "cpal"))]
    {
        if matches!(args.output, OutputMode::Auto)
            && std::fs::metadata(&args.device).is_err()
        {
            eprintln!(
                "spkrd: --output=auto: device {:?} not found and the CPAL fallback is not compiled in. \
                 Rebuild with default features to enable CPAL, or pass --output=freebsd-speaker --device <path>.",
                args.device
            );
            process::exit(1);
        }
    }

    let resolved = resolve_output(args.output, &args.device);

    info!(
        "Starting spkrd: port={}, retry_timeout={}s, max_melody_length={}, output={:?} (resolved={:?}), device={}, daemon={}, pidfile={}, debug={}",
        args.port,
        args.retry_timeout,
        args.max_melody_length,
        args.output,
        resolved,
        args.device,
        args.daemon,
        args.pidfile,
        args.debug
    );

    warn_unused_flags(&args, resolved, user_specified_output);

    let backend = build_backend(&args, resolved)?;

    if args.daemon {
        let daemonize = Daemonize::new()
            .pid_file(&args.pidfile)
            .working_directory("/");
        match daemonize.start() {
            Ok(_) => info!("Daemon started successfully"),
            Err(e) => {
                error!("Error starting daemon: {}", e);
                process::exit(1);
            }
        }
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    runtime.block_on(async move {
        match server::run(
            args.port,
            retry_timeout,
            backend,
            args.max_melody_length,
            args.debug,
        )
        .await
        {
            Ok(_) => {
                info!("Server shutdown completed");
                Ok(())
            }
            Err(e) => {
                error!("Server error: {}", e);
                Err(e)
            }
        }
    })
}
