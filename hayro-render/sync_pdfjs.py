import os
import json
import hashlib
import requests
import fnmatch
import shutil
from pathlib import Path
from typing import List, Dict, Any

class PDFJSSync:
    def __init__(self):
        self.script_dir = Path(__file__).resolve().parent
        self.pdfjs_test_dir = Path("/Users/lstampfl/Programming/GitHub/pdf.js/test")
        self.pdfjs_pdfs_dir = self.pdfjs_test_dir / "pdfs"
        self.pdfjs_manifest_path = self.pdfjs_test_dir / "test_manifest.json"
        
        # Our directories
        self.our_pdfs_dir = self.script_dir / "pdfs"
        self.our_downloads_dir = self.script_dir / "downloads"
        self.our_pdfjs_manifest_path = self.script_dir / "manifest_pdfjs.json"
        
        # Whitelist of tests to synchronize (support patterns with *)
        self.whitelist = [
            "calgray",
            "calrgb",
            "devicen",
            "mmtype1",
            "standard_fonts",
            "jbig2_symbol_offset",
            # "jbig2_huffman_1", (Already included in our test suite)
            # "jbig2_huffman_2", BLank test from what I can tell (the specific page range)
            "colorspace_atan",
            "colorspace_cos",
            "colorspace_sin",
            "issue2642", 
            # TODO: Takes very long?
            # "ccitt_EndOfBlock_false",
            "cid_cff",
            "cmykjpeg",
            "colors",
            "images_1bit_grayscale",
            "arabiccidtruetype-pdf",
            "clippath",
            "close-path-bug",
            "complex_ttf_font",
            "german-umlaut-r",
            "gradientfill",
            "helloworld-bad",
            "jp2k-resetprob",
            "rotated",
            # "ShowText-ShadingPattern",
            "simpletype3font",
            "bigboundingbox",
            # "Type3WordSpacing",
            # "xobject-image",
            # "ZapfDingbats",
            "IndexedCS_negative_and_high",  # Regular PDF test
            "operator-in-TJ-array",
            "issue4379",
        ]
        
    def load_pdfjs_manifest(self) -> List[Dict[str, Any]]:
        """Load the PDF.js test manifest."""
        if not self.pdfjs_manifest_path.exists():
            raise FileNotFoundError(f"PDF.js manifest not found: {self.pdfjs_manifest_path}")
            
        with open(self.pdfjs_manifest_path, 'r') as f:
            return json.load(f)
            
    def matches_whitelist(self, test_id: str) -> bool:
        """Check if test_id matches any pattern in the whitelist."""
        for pattern in self.whitelist:
            if fnmatch.fnmatch(test_id, pattern):
                return True
        return False
        
    def calculate_md5(self, file_path: Path) -> str:
        """Calculate MD5 hash of a file."""
        hash_md5 = hashlib.md5()
        with open(file_path, "rb") as f:
            for chunk in iter(lambda: f.read(4096), b""):
                hash_md5.update(chunk)
        return hash_md5.hexdigest()
        
    def download_pdf_from_link(self, link_file_path: Path, expected_md5: str, dest_name: str) -> bool:
        """Download PDF from a .link file and verify MD5."""
        dest_path = self.our_downloads_dir / f"{dest_name}.pdf"
        
        # Check if already downloaded with correct MD5
        if dest_path.exists():
            actual_md5 = self.calculate_md5(dest_path)
            if actual_md5 == expected_md5:
                print(f"✔ {dest_name} already downloaded with correct MD5")
                return True
            else:
                print(f"✘ {dest_name} exists but MD5 mismatch!")
                print(f"  Expected: {expected_md5}")
                print(f"  Actual:   {actual_md5}")
                raise RuntimeError(f"Existing file {dest_name} has wrong MD5. Expected: {expected_md5}, Actual: {actual_md5}")
        
        # Read URL from .link file
        url = link_file_path.read_text().strip()
        print(f"📥 Downloading {dest_name} from {url[:60]}...")
        
        try:
            response = requests.get(url, stream=True, timeout=60)
            response.raise_for_status()
            
            # Ensure downloads directory exists
            self.our_downloads_dir.mkdir(exist_ok=True)
            
            with open(dest_path, 'wb') as f:
                for chunk in response.iter_content(chunk_size=8192):
                    f.write(chunk)
            
            # Verify MD5
            actual_md5 = self.calculate_md5(dest_path)
            if actual_md5 == expected_md5:
                print(f"✔ Downloaded and verified MD5")
                return True
            else:
                print(f"✘ MD5 verification failed!")
                print(f"  Expected: {expected_md5}")
                print(f"  Actual:   {actual_md5}")
                dest_path.unlink()  # Delete the incorrect file
                raise RuntimeError(f"MD5 verification failed for {dest_name}. Expected: {expected_md5}, Actual: {actual_md5}")
                
        except requests.RequestException as e:
            print(f"✘ Failed to download {dest_name}: {e}")
            return False
            
    def copy_pdf_file(self, source_path: Path, dest_name: str) -> bool:
        """Copy a PDF file from PDF.js to our pdfs directory."""
        dest_path = self.our_pdfs_dir / f"{dest_name}.pdf"
        
        try:
            # Ensure pdfs directory exists
            self.our_pdfs_dir.mkdir(exist_ok=True)
            
            if dest_path.exists():
                print(f"✔ {dest_name}.pdf already exists, skipping copy")
                return True
                
            shutil.copy2(source_path, dest_path)
            print(f"📄 Copied {dest_name}.pdf")
            return True
            
        except Exception as e:
            print(f"✘ Failed to copy {dest_name}.pdf: {e}")
            return False
            
    def copy_link_file(self, source_path: Path, dest_name: str) -> bool:
        """Copy a .link file from PDF.js to our pdfs directory."""
        dest_path = self.our_pdfs_dir / f"{dest_name}.link"
        
        try:
            # Ensure pdfs directory exists
            self.our_pdfs_dir.mkdir(exist_ok=True)
            
            if dest_path.exists():
                print(f"✔ {dest_name}.link already exists, skipping copy")
                return True
                
            shutil.copy2(source_path, dest_path)
            print(f"🔗 Copied {dest_name}.link")
            return True
            
        except Exception as e:
            print(f"✘ Failed to copy {dest_name}.link: {e}")
            return False
            
    def convert_pdfjs_entry_to_our_format(self, entry: Dict[str, Any]) -> Dict[str, Any]:
        """Convert a PDF.js manifest entry to our manifest format."""
        our_entry = {
            "id": entry["id"],
            "file": f"pdfs/{entry['id']}.{'link' if entry.get('link', False) else 'pdf'}"
        }
        
        # Copy MD5 if it's a link
        if entry.get("link", False):
            our_entry["md5"] = entry["md5"]
            our_entry["link"] = True
            
        # Convert page range (PDF.js uses firstPage/lastPage, we use first_page/last_page)
        if "firstPage" in entry:
            our_entry["first_page"] = entry["firstPage"] - 1  # PDF.js is 1-indexed, we are 0-indexed
        if "lastPage" in entry:
            our_entry["last_page"] = entry["lastPage"] - 1    # PDF.js is 1-indexed, we are 0-indexed
            
        return our_entry
        
    def load_existing_pdfjs_manifest(self) -> List[Dict[str, Any]]:
        """Load our existing PDF.js manifest if it exists."""
        if self.our_pdfjs_manifest_path.exists():
            with open(self.our_pdfjs_manifest_path, 'r') as f:
                return json.load(f)
        return []
        
    def cleanup_removed_entries(self, existing_entries: List[Dict[str, Any]], current_whitelist_ids: set):
        """Remove files for entries that are no longer in the whitelist."""
        removed_count = 0
        
        for entry in existing_entries:
            entry_id = entry["id"]
            if entry_id not in current_whitelist_ids:
                print(f"🧹 Cleaning up {entry_id} (no longer in whitelist)...")
                
                # Remove PDF file
                pdf_path = self.our_pdfs_dir / f"{entry_id}.pdf"
                if pdf_path.exists():
                    pdf_path.unlink()
                    print(f"  ✔ Removed {entry_id}.pdf")
                
                # Remove link file
                link_path = self.our_pdfs_dir / f"{entry_id}.link"
                if link_path.exists():
                    link_path.unlink()
                    print(f"  ✔ Removed {entry_id}.link")
                    
                # Remove downloaded file
                download_path = self.our_downloads_dir / f"{entry_id}.pdf"
                if download_path.exists():
                    download_path.unlink()
                    print(f"  ✔ Removed downloaded {entry_id}.pdf")
                    
                removed_count += 1
                
        if removed_count > 0:
            print(f"🧹 Cleaned up {removed_count} removed entries")
        
    def sync(self):
        """Main synchronization function."""
        print("🚀 Starting PDF.js test synchronization...")
        print(f"📋 Whitelist patterns: {', '.join(self.whitelist)}")
        
        # Load existing manifest for cleanup
        existing_entries = self.load_existing_pdfjs_manifest()
        if existing_entries:
            print(f"📄 Found existing manifest with {len(existing_entries)} entries")
        
        # Load PDF.js manifest
        try:
            pdfjs_manifest = self.load_pdfjs_manifest()
            print(f"📄 Loaded PDF.js manifest with {len(pdfjs_manifest)} entries")
        except Exception as e:
            print(f"✘ Failed to load PDF.js manifest: {e}")
            return
            
        # Filter entries based on whitelist
        matching_entries = []
        for entry in pdfjs_manifest:
            if self.matches_whitelist(entry["id"]):
                matching_entries.append(entry)
                
        print(f"🎯 Found {len(matching_entries)} matching entries")
        
        # Get current whitelist IDs for cleanup
        current_whitelist_ids = {entry["id"] for entry in matching_entries}
        
        # Clean up entries that are no longer in whitelist
        if existing_entries:
            self.cleanup_removed_entries(existing_entries, current_whitelist_ids)
        
        if not matching_entries:
            print("ℹ No entries matched the whitelist patterns.")
            # Still write empty manifest to clear it
            with open(self.our_pdfjs_manifest_path, 'w') as f:
                json.dump([], f, indent=2)
            print("📄 Created empty manifest_pdfjs.json")
            return
            
        # Process each matching entry
        our_manifest_entries = []
        success_count = 0
        failed_count = 0
        
        for entry in matching_entries:
            entry_id = entry["id"]
            is_link = entry.get("link", False)
            
            print(f"\n📦 Processing {entry_id} ({'link' if is_link else 'pdf'})...")
            
            if is_link:
                # Handle .link files
                link_file_path = self.pdfjs_pdfs_dir / f"{entry_id}.pdf.link"
                if not link_file_path.exists():
                    print(f"✘ Link file not found: {link_file_path}")
                    failed_count += 1
                    continue
                    
                # Copy the .link file
                if not self.copy_link_file(link_file_path, entry_id):
                    failed_count += 1
                    continue
                    
                # Download the PDF
                if not self.download_pdf_from_link(link_file_path, entry["md5"], entry_id):
                    failed_count += 1
                    continue
                    
            else:
                # Handle regular PDF files
                pdf_file_path = self.pdfjs_pdfs_dir / f"{entry_id}.pdf"
                if not pdf_file_path.exists():
                    print(f"✘ PDF file not found: {pdf_file_path}")
                    failed_count += 1
                    continue
                    
                # Copy the PDF file
                if not self.copy_pdf_file(pdf_file_path, entry_id):
                    failed_count += 1
                    continue
                    
            # Convert to our manifest format
            our_entry = self.convert_pdfjs_entry_to_our_format(entry)
            our_manifest_entries.append(our_entry)
            success_count += 1
            
        # Write our PDF.js manifest
        try:
            with open(self.our_pdfjs_manifest_path, 'w') as f:
                json.dump(our_manifest_entries, f, indent=2)
                
            print(f"\n🎉 Synchronization complete!")
            print(f"📄 Created manifest_pdfjs.json with {len(our_manifest_entries)} entries")
            print(f"📊 Summary: {success_count} successful, {failed_count} failed")
            
        except Exception as e:
            print(f"✘ Failed to write manifest_pdfjs.json: {e}")

def main():
    syncer = PDFJSSync()
    syncer.sync()

if __name__ == '__main__':
    main() 