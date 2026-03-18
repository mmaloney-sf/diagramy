// Import the WebAssembly module
import init, { render_diagram_with_diagnostics } from './dist/diagramy.js';

// Initialize Ace Editor
const editor = ace.edit("editor");
editor.setTheme("ace/theme/monokai");
editor.session.setMode("ace/mode/text");
editor.setShowPrintMargin(false);
editor.setOption("showLineNumbers", true); // Always display line numbers

// Load saved keybinding preference from localStorage
const savedKeybinding = localStorage.getItem('aceKeybinding') || 'default';
const keybindingSelect = document.getElementById('keybinding-select');
keybindingSelect.value = savedKeybinding;

// Apply the saved keybinding
if (savedKeybinding === 'vim') {
    editor.setKeyboardHandler('ace/keyboard/vim');
} else {
    editor.setKeyboardHandler(null); // Default bindings
}

// Handle keybinding selection
keybindingSelect.addEventListener('change', function() {
    const selectedBinding = this.value;

    // Save to localStorage
    localStorage.setItem('aceKeybinding', selectedBinding);

    // Apply the keybinding
    if (selectedBinding === 'vim') {
        editor.setKeyboardHandler('ace/keyboard/vim');
    } else {
        editor.setKeyboardHandler(null); // Default bindings
    }
});

// Render diagram from editor content
function renderDiagram() {
    const svgContainer = document.getElementById('svg-container');
    const content = editor.getValue();

    // If content is empty, clear the container and annotations
    if (!content.trim()) {
        svgContainer.innerHTML = '';
        editor.session.clearAnnotations();
        return;
    }

    try {
        // Call the WebAssembly function to render the diagram with diagnostics
        const result = render_diagram_with_diagnostics(content);

        // Get diagnostics
        const diagnostics = result.diagnostics;

        // Convert diagnostics to Ace annotations
        const annotations = diagnostics.map(diag => ({
            row: diag.line - 1, // Ace uses 0-based line numbers
            column: diag.column - 1, // Ace uses 0-based column numbers
            text: diag.message,
            type: diag.severity // "error", "warning", or "info"
        }));

        // Set annotations in the editor
        editor.session.setAnnotations(annotations);

        // If we have SVG, display it
        const svg = result.svg;
        if (svg) {
            svgContainer.innerHTML = svg;
        } else {
            // Show error message in the right panel as well
            const errorMessages = diagnostics.map(d => `Line ${d.line}, Col ${d.column}: ${d.message}`).join('\n');
            svgContainer.innerHTML = `<div class="error-message">Errors:\n${errorMessages}</div>`;
        }
    } catch (error) {
        // Display error message
        svgContainer.innerHTML = `<div class="error-message">Error rendering diagram:\n${error}</div>`;
        editor.session.clearAnnotations();
    }
}

// Initialize WebAssembly and set up editor listener
async function initApp() {
    try {
        // Initialize the WebAssembly module
        await init();

        // Listen for changes in the editor
        editor.session.on('change', function() {
            renderDiagram();
        });

        // Initial render
        renderDiagram();
    } catch (error) {
        console.error('Failed to initialize WebAssembly:', error);
        document.getElementById('svg-container').innerHTML =
            `<div class="error-message">Failed to initialize WebAssembly:\n${error}</div>`;
    }
}

// Start the application
initApp();

