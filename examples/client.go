// Go client example for SPKRD server

package main

import (
	"bytes"
	"fmt"
	"io"
	"net/http"
	"os"
)

func main() {
	if len(os.Args) != 3 {
		fmt.Fprintf(os.Stderr, "Usage: %s <server_url> <melody>\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "Example: %s http://192.168.1.100:8080 \"cdefgab\"\n", os.Args[0])
		os.Exit(1)
	}

	serverURL := os.Args[1]
	melody := os.Args[2]

	url := fmt.Sprintf("%s/play", serverURL)
	
	fmt.Printf("Playing melody: %s\n", melody)
	fmt.Printf("Server: %s\n", url)

	// Create HTTP request
	req, err := http.NewRequest("PUT", url, bytes.NewBufferString(melody))
	if err != nil {
		fmt.Fprintf(os.Stderr, "✗ Failed to create request: %v\n", err)
		os.Exit(1)
	}

	// Send request
	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		fmt.Fprintf(os.Stderr, "✗ Connection error: %v\n", err)
		os.Exit(1)
	}
	defer resp.Body.Close()

	// Handle response
	switch resp.StatusCode {
	case 200:
		fmt.Println("✓ Melody played successfully")
	case 400:
		body, _ := io.ReadAll(resp.Body)
		fmt.Fprintf(os.Stderr, "✗ Invalid melody: %s\n", string(body))
		os.Exit(1)
	case 503:
		body, _ := io.ReadAll(resp.Body)
		fmt.Fprintf(os.Stderr, "✗ Device busy: %s\n", string(body))
		os.Exit(1)
	case 500:
		body, _ := io.ReadAll(resp.Body)
		fmt.Fprintf(os.Stderr, "✗ Server error: %s\n", string(body))
		os.Exit(1)
	default:
		fmt.Fprintf(os.Stderr, "✗ Unexpected response: HTTP %d\n", resp.StatusCode)
		os.Exit(1)
	}
}