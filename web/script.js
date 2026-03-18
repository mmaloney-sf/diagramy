// Import the WebAssembly module
import init, { render_diagram } from './dist/diagramy.js';

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

    // If content is empty, clear the container
    if (!content.trim()) {
        svgContainer.innerHTML = '';
        return;
    }

    try {
        // Call the WebAssembly function to render the diagram
        const svgString = render_diagram(content);

        // Display the SVG
        svgContainer.innerHTML = svgString;
    } catch (error) {
        // Display error message
        svgContainer.innerHTML = `<div class="error-message">Error rendering diagram:\n${error}</div>`;
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

