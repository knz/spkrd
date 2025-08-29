# SPKRC Config File Flexibility Improvements

## Task Specification
Enhance the Rust client's config file (~/.spkrc) parsing to be more flexible with URL formats:
- Auto-add 'http://' prefix if missing
- Auto-add default port if not specified
- Handle trailing '/' properly to avoid path duplication

## Requirements Analysis
Based on user request, need to enhance ~/.spkrc config file parsing to handle:
1. Auto-add 'http://' prefix if missing
2. Auto-add default port (:1111) if not specified  
3. Handle trailing '/' to avoid path duplication when appending '/play'

## Current Implementation
- Config file read in `read_server_from_config()` function (client.rs:30-36)
- Simply reads file content and trims whitespace
- No URL parsing or normalization
- URL used directly with `format!("{}/play", server_url)` (client.rs:67)
- Default port is 1111 based on server implementation

## Requirements Clarification
1. **Scheme handling**: Keep existing scheme if present (http:// or https://), only add http:// if missing
2. **Port handling**: Only add :1111 if no port specified at all (e.g., `server` → `http://server:1111`, but `server:3000` → `http://server:3000`)
3. **Trailing slash**: Remove trailing slashes from config, always append /play
4. **Error handling**: Basic string manipulation only, no complex URL validation
5. **Backward compatibility**: Must remain fully compatible with existing complete URLs

## Implementation Plan
1. Create `normalize_server_url()` function to handle URL processing
2. Add scheme prefix if missing (default to http://)
3. Add default port :1111 if no port present
4. Remove trailing slashes
5. Update `get_server_url()` to use normalized URL
6. Update URL construction to use normalized base + "/play"

## Implementation Details
Created `normalize_server_url()` function with following logic:
1. **Scheme handling**: Checks for existing http:// or https:// prefix, adds http:// if missing
2. **Trailing slash removal**: Uses `trim_end_matches('/')` to remove all trailing slashes
3. **Port detection**: Checks if hostname contains ':' after scheme, adds :1111 if no port present
4. **Integration**: Updated `get_server_url()` to normalize both config file and CLI inputs

## Files Modified
- examples/client.rs: Added normalize_server_url() function and updated get_server_url()

## Testing Results
Verified URL normalization with multiple test cases:
- "server" → "http://server:1111/play" ✓
- "localhost:3000/" → "http://localhost:3000/play" ✓ 
- "https://example.com:8080/" → "https://example.com:8080/play" ✓
- CLI override: "localhost:1111/" → "http://localhost:1111/play" ✓

## Current Status
- Implementation completed and tested successfully
- All requirements met: scheme prefix, default port, trailing slash handling
- Backward compatibility maintained