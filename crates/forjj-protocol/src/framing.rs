//! Length-prefixed framing for protocol messages.
//!
//! Format: [4-byte big-endian length][payload]
//! Maximum message size: 16 MB

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Maximum message size (16 MB)
pub const MAX_MESSAGE_SIZE: u32 = 16 * 1024 * 1024;

/// Framing errors.
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("message too large: {size} bytes (max {MAX_MESSAGE_SIZE})")]
    MessageTooLarge { size: u32 },

    #[error("unexpected end of stream")]
    UnexpectedEof,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Write a length-prefixed frame.
pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    data: &[u8],
) -> Result<(), FrameError> {
    let len = data.len() as u32;
    if len > MAX_MESSAGE_SIZE {
        return Err(FrameError::MessageTooLarge { size: len });
    }

    writer.write_u32(len).await?;
    writer.write_all(data).await?;
    writer.flush().await?;

    Ok(())
}

/// Read a length-prefixed frame, allocating memory for it.
pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Vec<u8>, FrameError> {
    let len = match reader.read_u32().await {
        Ok(len) => len,
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(FrameError::UnexpectedEof);
        }
        Err(e) => return Err(FrameError::Io(e)),
    };

    if len > MAX_MESSAGE_SIZE {
        return Err(FrameError::MessageTooLarge { size: len });
    }

    let mut buffer = vec![0u8; len as usize];
    reader.read_exact(&mut buffer).await?;

    Ok(buffer)
}

/// Read a frame into a provided buffer.
pub async fn read_frame_into<R: AsyncRead + Unpin>(
    reader: &mut R,
    buffer: &mut [u8],
) -> Result<usize, FrameError> {
    let len = match reader.read_u32().await {
        Ok(len) => len,
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(FrameError::UnexpectedEof);
        }
        Err(e) => return Err(FrameError::Io(e)),
    };

    if len > MAX_MESSAGE_SIZE {
        return Err(FrameError::MessageTooLarge { size: len });
    }

    let len = len as usize;
    if len > buffer.len() {
        return Err(FrameError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "buffer too small",
        )));
    }

    reader.read_exact(&mut buffer[..len]).await?;

    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_frame_roundtrip() {
        let message = b"hello, forjj protocol!";

        let mut buffer = Vec::new();
        write_frame(&mut buffer, message).await.unwrap();

        let mut cursor = Cursor::new(buffer);
        let result = read_frame(&mut cursor).await.unwrap();

        assert_eq!(result, message);
    }

    #[tokio::test]
    async fn test_frame_too_large() {
        let large_data = vec![0u8; (MAX_MESSAGE_SIZE + 1) as usize];
        let mut buffer = Vec::new();

        let result = write_frame(&mut buffer, &large_data).await;
        assert!(matches!(result, Err(FrameError::MessageTooLarge { .. })));
    }

    #[tokio::test]
    async fn test_read_frame_into() {
        let message = b"test message";

        let mut buffer = Vec::new();
        write_frame(&mut buffer, message).await.unwrap();

        let mut cursor = Cursor::new(buffer);
        let mut read_buffer = [0u8; 256];
        let len = read_frame_into(&mut cursor, &mut read_buffer)
            .await
            .unwrap();

        assert_eq!(&read_buffer[..len], message);
    }
}
