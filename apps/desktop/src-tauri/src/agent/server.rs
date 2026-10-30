use super::{
    policy::{AgentPolicy, Clock, PeerInfo, SystemClock},
    protocol::*,
};
use crate::state::AppState;
use std::{
    io::{self, Write},
    os::{
        fd::AsRawFd,
        unix::{
            fs::{FileTypeExt, PermissionsExt},
            net::{UnixListener, UnixStream},
        },
    },
    path::PathBuf,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
pub struct AgentHandle {
    pub policy: Arc<AgentPolicy>,
    running: Arc<AtomicBool>,
    worker: Mutex<Option<(PathBuf, std::thread::JoinHandle<()>)>>,
}
impl Default for AgentHandle {
    fn default() -> Self {
        Self::new(Arc::new(SystemClock))
    }
}
impl AgentHandle {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            policy: Arc::new(AgentPolicy::new(clock)),
            running: Arc::new(AtomicBool::new(false)),
            worker: Mutex::new(None),
        }
    }
    pub fn running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
    pub fn start(&self, st: Arc<AppState>, socket: PathBuf) -> io::Result<()> {
        let mut worker = self.worker.lock().unwrap_or_else(|e| e.into_inner());
        if worker.is_some() {
            return Err(io::ErrorKind::AddrInUse.into());
        }
        let parent = socket.parent().ok_or(io::ErrorKind::InvalidInput)?;
        std::fs::create_dir_all(parent)?;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        if socket.symlink_metadata().is_ok() {
            if UnixStream::connect(&socket).is_ok() {
                return Err(io::ErrorKind::AddrInUse.into());
            }
            if !socket.symlink_metadata()?.file_type().is_socket() {
                return Err(io::ErrorKind::AlreadyExists.into());
            }
            std::fs::remove_file(&socket)?;
        }
        let listener = UnixListener::bind(&socket)?;
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))?;
        listener.set_nonblocking(true)?;
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let weak = Arc::downgrade(&st);
        let thread = std::thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let weak = weak.clone();
                        let running = running.clone();
                        std::thread::spawn(move || connection(stream, weak, running));
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10))
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
        });
        *worker = Some((socket, thread));
        Ok(())
    }
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.policy.clear_trust();
        let mut worker = self.worker.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((path, thread)) = worker.take() {
            let _ = thread.join();
            let _ = std::fs::remove_file(path);
        }
    }
}
impl Drop for AgentHandle {
    fn drop(&mut self) {
        self.stop();
    }
}
pub(super) fn connection(mut stream: UnixStream, st: Weak<AppState>, running: Arc<AtomicBool>) {
    // Darwin inherits O_NONBLOCK from the listener. Each worker uses blocking
    // framed I/O; a pause between requests must not be treated as disconnect.
    if stream.set_nonblocking(false).is_err() {
        return;
    }
    let Some(peer) = peer_info(&stream) else {
        return;
    };
    if peer.uid != unsafe { libc::geteuid() } {
        return;
    }
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    while running.load(Ordering::SeqCst) {
        let Ok(Some(body)) = read_frame(&mut stream) else {
            return;
        };
        if !running.load(Ordering::SeqCst) {
            return;
        }
        let Some(st) = st.upgrade() else {
            return;
        };
        let response = decode_request(&body)
            .map(|r| st.agent.policy.handle(&st, r, &peer))
            .unwrap_or(Response::Failure);
        if stream.write_all(&encode_response(&response)).is_err() {
            return;
        }
    }
}
fn peer_info(stream: &UnixStream) -> Option<PeerInfo> {
    let fd = stream.as_raw_fd();
    let mut uid = 0;
    let mut gid = 0;
    if unsafe { libc::getpeereid(fd, &mut uid, &mut gid) } != 0 {
        return None;
    }
    let mut pid = 0i32;
    let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
    let pid = if unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_LOCAL,
            libc::LOCAL_PEERPID,
            (&mut pid as *mut i32).cast(),
            &mut len,
        )
    } == 0
    {
        Some(pid)
    } else {
        None
    };
    let process = pid.and_then(|p| {
        let mut path = vec![0u8; 4096];
        let n = unsafe { libc::proc_pidpath(p, path.as_mut_ptr().cast(), path.len() as u32) };
        if n <= 0 {
            return None;
        }
        let end = path.iter().position(|b| *b == 0).unwrap_or(path.len());
        std::path::Path::new(std::str::from_utf8(&path[..end]).ok()?)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
    });
    Some(PeerInfo { uid, pid, process })
}
