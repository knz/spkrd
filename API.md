# SPKRD API Documentation

FreeBSD Speaker Device Network Server API

## Overview

SPKRD provides HTTP access to FreeBSD's `/dev/speaker` device for remote melody playback. The server handles device concurrency automatically by retrying requests when the device is busy.

## Base URL

```
http://your-server:1111
```

## Endpoints

### PUT /play

Plays a melody on the FreeBSD speaker device.

**Request:**
- Method: `PUT`
- Path: `/play`
- Content-Type: `text/plain` (optional)
- Body: Melody string in FreeBSD speaker format

**Request Body:**
- Maximum 1000 characters
- FreeBSD speaker melody format (see `man 4 speaker`)
- Example: `"cdefgab"`

**Response:**
- Success: HTTP 200 with empty body
- Validation Error: HTTP 400 with error message
- Device Busy (timeout): HTTP 503 with error message  
- Server Error: HTTP 500 with error message

## Examples

### Play a simple melody
```bash
curl -X PUT http://localhost:1111/play -d "cdefgab"
```

### Play a more complex melody
```bash
curl -X PUT http://localhost:1111/play -d "t120l8cdegreg"
```

## Error Responses

| Status Code | Meaning | Example |
|-------------|---------|---------|
| 200 | Success | Empty body |
| 400 | Invalid melody | "Melody exceeds 1000 characters" |
| 503 | Device busy/timeout | "Device busy - request timed out" |
| 500 | Server error | "Device error: Permission denied" |

## Melody Format

The server accepts melodies in FreeBSD speaker format. Common syntax:

- **Notes:** `a`, `b`, `c`, `d`, `e`, `f`, `g` (with optional `#` or `+` for sharp)
- **Octaves:** `o1` to `o7` (default o4)
- **Length:** `l1`, `l2`, `l4`, `l8`, `l16`, `l32` (whole, half, quarter, etc.)
- **Tempo:** `t60` to `t255` (beats per minute)
- **Pause:** `p` followed by length
- **Repeat:** `.` after note extends by half

Example: `"t120l4 c d e f g a b o5c"`

## Server Configuration

```bash
spkrd --port 1111 --retry-timeout 30 --device /dev/speaker
```

Options:
- `--port`: Server port (default: 1111)
- `--retry-timeout`: Device retry timeout in seconds (default: 30)
- `--device`: Path to speaker device (default: /dev/speaker)