use {
    std::{
        future::Future,
        io::SeekFrom,
        path::PathBuf,
        pin::Pin,
        time::Duration,
    },
    tokio::{
        io::{
            self,
            AsyncBufReadExt as _,
            AsyncSeekExt as _,
            BufReader,
        },
        sync::mpsc,
        time::sleep,
    },
    wheel::fs::File,
};
#[cfg(unix)] use std::os::unix::fs::MetadataExt;
#[cfg(windows)] use {
    std::{
        ffi::c_void,
        mem::{
            self,
            MaybeUninit,
        },
        os::windows::io::AsRawHandle as _,
    },
    wheel::traits::IoResultExt as _,
    windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            FILE_ID_INFO,
            FileIdInfo,
            GetFileInformationByHandleEx,
        },
    },
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)] Custom(Box<dyn std::error::Error + Send + Sync>),
    #[error(transparent)] Io(#[from] io::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
}

const DEFAULT_ROTATION_CHECK_WAIT: Duration = Duration::from_millis(100);
const DEFAULT_NOT_ROTATED_WAIT: Duration = Duration::from_millis(50);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub struct Line(pub usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
struct Pos(u64);

#[cfg(unix)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
struct FileId(u64);

#[cfg(windows)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct FileId(FILE_ID_INFO);

#[derive(Debug, Clone)]
pub struct Chaser {
    line: Line,
    path: PathBuf,
}

#[derive(Debug)]
struct Chasing<'a> {
    chaser: &'a mut Chaser,
    file_id: FileId,
    reader: BufReader<File>,
    buffer: String,
    line: Line,
    pos: Pos,
}

impl Chaser {
    pub fn new(path: impl Into<PathBuf>, line: Line) -> Self {
        Self {
            path: path.into(),
            line,
        }
    }

    pub fn run(mut self) -> mpsc::Receiver<Result<String, Error>> {
        let (tx, rx) = mpsc::channel(1);
        tokio::spawn(async move {
            match self.run_inner(|line| {
                let line = line.to_owned();
                async {
                    tx.send(Ok(line)).await.is_ok()
                }
            }).await {
                Ok(()) => {}
                Err(e) => { let _ = tx.send(Err(e)).await; }
            }
        });
        rx
    }

    async fn run_inner<Fut: Future<Output = bool> + Send>(&mut self, mut f: impl FnMut(&str) -> Fut + Send) -> Result<(), Error> {
        let (file, file_id) = try_until_success::<_, Error, _>(|| async {
            let file = File::open(&self.path).await?;
            let file_id = get_file_id(&file).await?;
            Ok((file, file_id))
        }).await;
        // Create a BufReader and skip to the proper line number while
        // keeping track of byte-position
        let mut reader = BufReader::new(file);
        let mut current_line = Line(0);
        let mut current_pos = Pos(0);
        let mut buffer = String::new();
        'skip_to_line: while current_line < self.line {
            let read_bytes = reader.read_line(&mut buffer).await? as u64;
            if read_bytes > 0 {
                current_pos.0 += read_bytes;
                current_line.0 += 1;
                buffer.clear();
                reader.seek(SeekFrom::Start(current_pos.0)).await?;
            } else {
                break 'skip_to_line;
            }
        }

        let mut running = Chasing {
            chaser: self,
            file_id,
            reader,
            buffer,
            pos: current_pos,
            line: current_line,
        };
        chase(&mut running, &mut f, false).await
    }
}

fn chase<'a, 'b: 'a, 'c: 'a, 'd: 'a, Fut: Future<Output = bool> + Send>(running: &'b mut Chasing<'c>, f: &'d mut (impl FnMut(&str) -> Fut + Send), grabbing_remainder: bool) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>> {
    Box::pin(async move {
        'reading: loop {
            'read_to_eof: loop {
                let bytes_read = running.reader.read_line(&mut running.buffer).await?;
                if bytes_read > 0 {
                    if !f(running.buffer.trim_end_matches('\n')).await {
                        return Ok(())
                    }
                    running.buffer.clear();
                    running.line.0 += 1;
                    running.pos.0 += bytes_read as u64;
                    running.reader.seek(SeekFrom::Start(running.pos.0)).await?;
                } else {
                    break 'read_to_eof // no bytes read -> EOF
                }
            }

            if grabbing_remainder {
                break 'reading
            } else {
                let rotation_status = try_until_success(|| check_rotation_status(running)).await;
                match rotation_status {
                    RotationStatus::Rotated {
                        file: new_file,
                        file_id: new_file_id,
                    } => {
                        // Read the rest of the same file
                        chase(running, f, true).await?;
                        // Restart reading loop, but read from the top
                        running.line = Line(0);
                        running.pos = Pos(0);
                        running.file_id = new_file_id;
                        running.reader = BufReader::new(new_file);
                        continue 'reading
                    }
                    RotationStatus::NotRotated => {
                        sleep(DEFAULT_NOT_ROTATED_WAIT).await;
                        continue 'reading
                    }
                }
            }
        }
        Ok(())
    })
}

fn check_rotation_status(running: &mut Chasing<'_>) -> impl Future<Output = wheel::Result<RotationStatus>> {
    let path = running.chaser.path.clone();
    let running_file_id = running.file_id;
    async move {
        let file = File::open(path).await?;
        let file_id = get_file_id(&file).await?;
        if file_id != running_file_id {
            Ok(RotationStatus::Rotated { file, file_id })
        } else {
            Ok(RotationStatus::NotRotated)
        }
    }
}

async fn try_until_success<T, E, Fut: Future<Output = Result<T, E>>>(mut f: impl FnMut() -> Fut) -> T {
    loop {
        if let Ok(value) = f().await {
            break value
        } else {
            sleep(DEFAULT_ROTATION_CHECK_WAIT).await;
        }
    }
}

#[cfg(unix)]
async fn get_file_id(file: &File) -> wheel::Result<FileId> {
    let meta = file.metadata().await?;
    Ok(FileId(meta.ino()))
}

#[cfg(windows)]
async fn get_file_id(file: &File) -> wheel::Result<FileId> {
    let mut file_id_info = MaybeUninit::uninit();
    // SAFETY: GetFileInformationByHandleEx does not specify safety criteria
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle() as isize),
            FileIdInfo,
            file_id_info.as_mut_ptr() as *mut c_void,
            mem::size_of::<FILE_ID_INFO>().try_into().unwrap(),
        ).map_err(io::Error::from).at_unknown()?; //TODO get path from file
    }
    // SAFETY: file_id_info is initialized by GetFileInformationByHandleEx
    unsafe {
        Ok(FileId(file_id_info.assume_init()))
    }
}

enum RotationStatus {
    Rotated { file: File, file_id: FileId },
    NotRotated,
}
