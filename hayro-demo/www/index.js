import init, { PdfViewer } from './hayro_demo.js';

let pdfViewer = null;
let currentImage = null;

async function run() {
    await init();

    const dropZone = document.getElementById('drop-zone');
    const fileInput = document.getElementById('file-input');
    const fileSelector = document.getElementById('file-selector');
    const viewer = document.getElementById('viewer');
    const canvas = document.getElementById('pdf-canvas');
    const prevButton = document.getElementById('prev-page');
    const nextButton = document.getElementById('next-page');
    const pageInfo = document.getElementById('page-info');
    const pageInput = document.getElementById('page-input');
    const dropOverlay = document.getElementById('drop-overlay');

    // Make drop zone clickable to open file dialog
    dropZone.addEventListener('click', () => {
        fileInput.click();
    });

    // Prevent default drag behaviors
    const preventDefaults = (e) => {
        e.preventDefault();
        e.stopPropagation();
    };

    ['dragenter', 'dragover', 'dragleave', 'drop'].forEach(eventName => {
        document.addEventListener(eventName, preventDefaults, false);
    });

    // Function to handle PDF files from drag and drop or file input
    function handlePDFDrop(e, files) {
        preventDefaults(e);
        
        // Hide drop overlays
        dropZone.classList.remove('dragover');
        dropOverlay.style.display = 'none';
        
        if (files.length > 0) {
            handleFile(files[0]);
        }
    }

    // Highlight drop zones when item is dragged over them
    ['dragenter', 'dragover'].forEach(eventName => {
        dropZone.addEventListener(eventName, (e) => {
            preventDefaults(e);
            dropZone.classList.add('dragover');
        }, false);
        
        // Show drop overlay when dragging over viewer
        viewer.addEventListener(eventName, (e) => {
            preventDefaults(e);
            if (pdfViewer) {
                dropOverlay.style.display = 'flex';
            }
        }, false);
    });

    // Remove drag styling when leaving
    ['dragleave'].forEach(eventName => {
        dropZone.addEventListener(eventName, (e) => {
            preventDefaults(e);
            dropZone.classList.remove('dragover');
        }, false);
        
        viewer.addEventListener(eventName, (e) => {
            preventDefaults(e);
            // Only hide if we're leaving the viewer completely
            if (!viewer.contains(e.relatedTarget)) {
                dropOverlay.style.display = 'none';
            }
        }, false);
    });

    // Handle dropped files
    dropZone.addEventListener('drop', (e) => {
        handlePDFDrop(e, e.dataTransfer.files);
    }, false);

    viewer.addEventListener('drop', (e) => {
        handlePDFDrop(e, e.dataTransfer.files);
    }, false);

    // Handle file selection
    fileInput.addEventListener('change', (e) => {
        if (e.target.files.length > 0) {
            handleFile(e.target.files[0]);
        }
    });

    // Navigation handlers
    prevButton.addEventListener('click', () => {
        if (pdfViewer && pdfViewer.previous_page()) {
            renderCurrentPage();
        }
    });

    nextButton.addEventListener('click', () => {
        if (pdfViewer && pdfViewer.next_page()) {
            renderCurrentPage();
        }
    });

    // Page input handler
    pageInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            const pageNum = parseInt(pageInput.value);
            if (pdfViewer && pdfViewer.set_page(pageNum)) {
                renderCurrentPage();
            } else {
                pageInput.value = pdfViewer ? pdfViewer.get_current_page() : 1;
            }
        }
    });

    // Handle file loading
    async function handleFile(file) {
        if (file.type !== 'application/pdf') {
            alert('Please select a PDF file.');
            return;
        }
        
        try {
            const arrayBuffer = await file.arrayBuffer();
            const uint8Array = new Uint8Array(arrayBuffer);
            await loadPDFData(uint8Array);
        } catch (error) {
            console.error('Error reading file:', error);
            alert('Error reading file: ' + error.message);
        }
    }

    // Load PDF data
    async function loadPDFData(uint8Array) {
        try {
            pdfViewer = new PdfViewer();
            await pdfViewer.load_pdf(uint8Array);
            
            // Switch to viewer
            fileSelector.style.display = 'none';
            viewer.style.display = 'block';
            
            renderCurrentPage();
        } catch (error) {
            console.error('Error loading PDFs:', error);
        }
    }

    // Render current page
    async function renderCurrentPage() {
        if (!pdfViewer) return;
        
        try {
            const pngData = pdfViewer.render_current_page();
            
            // Create blob and load image
            const blob = new Blob([pngData], { type: 'image/png' });
            const imageUrl = URL.createObjectURL(blob);
            
            const img = new Image();
            img.onload = () => {
                currentImage = img;
                drawImage();
                updatePageInfo();
                
                // Clean up the URL
                URL.revokeObjectURL(imageUrl);
            };
            img.onerror = () => {
                console.error('Error loading rendered image');
            };
            img.src = imageUrl;
            
        } catch (error) {
            console.error('Error rendering page:', error);
        }
    }

    // Draw image on canvas
    function drawImage() {
        if (!currentImage) return;

        const ctx = canvas.getContext('2d');
        
        // Get viewport dimensions (minus some padding for controls)
        const viewportWidth = window.innerWidth;
        const viewportHeight = window.innerHeight - 120; // Reserve space for controls
        
        // Calculate scale to fit image within viewport while maintaining aspect ratio
        const scaleX = viewportWidth / currentImage.width;
        const scaleY = viewportHeight / currentImage.height;
        const scale = Math.min(scaleX, scaleY, 1); // Don't scale up beyond original size
        
        // Calculate scaled dimensions
        const scaledWidth = currentImage.width * scale;
        const scaledHeight = currentImage.height * scale;
        
        // Set canvas size to scaled dimensions
        canvas.width = scaledWidth;
        canvas.height = scaledHeight;
        
        // Clear and draw scaled image
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.drawImage(currentImage, 0, 0, scaledWidth, scaledHeight);
    }

    // Update page information
    function updatePageInfo() {
        if (!pdfViewer) return;
        
        const currentPage = pdfViewer.get_current_page();
        const totalPages = pdfViewer.get_total_pages();
        
        pageInfo.textContent = `${currentPage} / ${totalPages}`;
        pageInput.value = currentPage;
        pageInput.max = totalPages;
        
        // Update button states
        prevButton.disabled = currentPage === 1;
        nextButton.disabled = currentPage === totalPages;
    }

    // Keyboard navigation
    document.addEventListener('keydown', (e) => {
        if (!pdfViewer) return;
        
        switch (e.key) {
            case 'ArrowLeft':
            case 'ArrowUp':
                e.preventDefault();
                if (pdfViewer.previous_page()) {
                    renderCurrentPage();
                }
                break;
            case 'ArrowRight':
            case 'ArrowDown':
                e.preventDefault();
                if (pdfViewer.next_page()) {
                    renderCurrentPage();
                }
                break;
        }
    });

    // Handle window resize to refit PDF
    window.addEventListener('resize', () => {
        if (currentImage) {
            drawImage();
        }
    });

    // Setup log window
    setupLogWindow();
}

// Setup log window functionality
function setupLogWindow() {
    const logContent = document.getElementById('log-content');
    const clearLogsButton = document.getElementById('clear-logs');

    // Clear logs on page load
    logContent.innerHTML = '';

    // Clear logs button
    clearLogsButton.addEventListener('click', () => {
        logContent.innerHTML = '';
    });

    // Function to add log entry
    window.addLogEntry = function(level, message) {
        const logEntry = document.createElement('div');
        logEntry.className = `log-entry ${level}`;
        
        const timestamp = new Date().toLocaleTimeString();
        logEntry.innerHTML = `<span class="log-timestamp">[${timestamp}]</span>${message}`;
        
        logContent.appendChild(logEntry);
        logContent.scrollTop = logContent.scrollHeight; // Auto-scroll to bottom
    };

    // Override console methods to also log to our window
    const originalConsoleWarn = console.warn;
    const originalConsoleError = console.error;
    const originalConsoleLog = console.log;
    const originalConsoleInfo = console.info;

    console.warn = function(...args) {
        originalConsoleWarn.apply(console, args);
        window.addLogEntry('warn', args.join(' '));
    };

    console.error = function(...args) {
        originalConsoleError.apply(console, args);
        window.addLogEntry('error', args.join(' '));
    };

    console.log = function(...args) {
        originalConsoleLog.apply(console, args);
        window.addLogEntry('info', args.join(' '));
    };

    console.info = function(...args) {
        originalConsoleInfo.apply(console, args);
        window.addLogEntry('info', args.join(' '));
    };
}

// Start the application
run().catch(console.error); 
