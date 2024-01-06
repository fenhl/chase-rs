//! Holds a synchronous implementation of file following.

use crate::data::*;

use std::{
    fs::File,
    io::{
        self,
        BufReader,
        SeekFrom,
        prelude::*,
    },
    thread::sleep,
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
    windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            FILE_ID_INFO,
            FileIdInfo,
            GetFileInformationByHandleEx,
        },
    },
};

impl Chaser {
    pub(crate) fn run<F>(&mut self, mut f: F) -> Result<(), crate::Error>
    where
        F: FnMut(&str) -> Result<(), crate::Error>,
    {
        let (file, file_id) = try_until::<_, crate::Error, _>(|| {
            let file = File::open(&self.path)?;
            let file_id = get_file_id(&file)?;
            Ok((file, file_id))
        })?;
        // Create a BufReader and skip to the proper line number while
        // keeping track of byte-position
        let mut reader = BufReader::new(file);
        let mut current_line = Line(0);
        let mut current_pos = Pos(0);
        let mut buffer = String::new();
        'skip_to_line: while current_line < self.line {
            let read_bytes = reader.read_line(&mut buffer)? as u64;
            if read_bytes > 0 {
                current_pos.0 += read_bytes;
                current_line.0 += 1;
                buffer.clear();
                reader.seek(SeekFrom::Start(current_pos.0))?;
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
        chase(&mut running, &mut f, false)
    }
}

fn chase<F>(running: &mut Chasing<'_>, f: &mut F, grabbing_remainder: bool) -> Result<(), crate::Error>
where
    F: FnMut(&str) -> Result<(), crate::Error>,
{
    'reading: loop {
        'read_to_eof: loop {
            let bytes_read = running.reader.read_line(&mut running.buffer)?;
            if bytes_read > 0 {
                f(running.buffer.trim_end_matches('\n'))?;
                running.buffer.clear();
                running.line.0 += 1;
                running.pos.0 += bytes_read as u64;
                running.reader.seek(SeekFrom::Start(running.pos.0))?;
            } else {
                break 'read_to_eof; // no bytes read -> EOF
            }
        }

        if grabbing_remainder {
            break 'reading;
        } else {
            let rotation_status = try_until(|| check_rotation_status(running))?;
            match rotation_status {
                RotationStatus::Rotated {
                    file: new_file,
                    file_id: new_file_id,
                } => {
                    // Read the rest of the same file
                    chase(running, f, true)?;
                    // Restart reading loop, but read from the top
                    running.line = Line(0);
                    running.pos = Pos(0);
                    running.file_id = new_file_id;
                    running.reader = BufReader::new(new_file);
                    continue 'reading;
                }
                RotationStatus::NotRotated => {
                    sleep(DEFAULT_NOT_ROTATED_WAIT);
                    continue 'reading;
                }
            }
        }
    }
    Ok(())
}

fn check_rotation_status(running: &mut Chasing<'_>) -> Result<RotationStatus, io::Error> {
    let file = File::open(&running.chaser.path)?;
    let file_id = get_file_id(&file)?;
    if file_id != running.file_id {
        Ok(RotationStatus::Rotated { file, file_id })
    } else {
        Ok(RotationStatus::NotRotated)
    }
}

// Will go at least once, max attempts set to None means try until successful
fn try_until<R, E, F>(
    mut f: F,
) -> Result<R, E>
where
    F: FnMut() -> Result<R, E>,
{
    loop {
        let current_try = f();
        if current_try.is_err() {
            sleep(DEFAULT_ROTATION_CHECK_WAIT);
            continue
        } else {
            return current_try
        }
    }
}

#[cfg(unix)]
fn get_file_id(file: &File) -> Result<FileId, io::Error> {
    let meta = file.metadata()?;
    Ok(FileId(meta.ino()))
}

#[cfg(windows)]
fn get_file_id(file: &File) -> Result<FileId, io::Error> {
    let mut file_id_info = MaybeUninit::uninit();
    // SAFETY: GetFileInformationByHandleEx does not specify safety criteria
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle() as isize),
            FileIdInfo,
            file_id_info.as_mut_ptr() as *mut c_void,
            mem::size_of::<FILE_ID_INFO>().try_into().unwrap(),
        )?;
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
