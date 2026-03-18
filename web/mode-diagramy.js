// Custom Ace Editor mode for Diagramy (.dgmy) files
ace.define("ace/mode/diagramy", ["require", "exports", "module", "ace/lib/oop", "ace/mode/text", "ace/mode/text_highlight_rules"], function(require, exports, module) {
    "use strict";

    var oop = require("../lib/oop");
    var TextMode = require("./text").Mode;
    var TextHighlightRules = require("./text_highlight_rules").TextHighlightRules;

    var DiagramyHighlightRules = function() {
        // Keywords
        var keywords = (
            "diagram|box|port|arrow|label|is|at|to|on|dim"
        );

        // Property names
        var properties = (
            "version|width|height|color|title|top|grid|text|borderStyle|margin|bold|debug"
        );

        // Color values
        var colors = (
            "transparent|red|blue|green|yellow|orange|purple|pink|cyan|magenta|" +
            "lime|teal|indigo|brown|gray|grey|black|white|navy|maroon|olive"
        );

        // Border styles
        var borderStyles = (
            "solid|none|dotted|dashed"
        );

        this.$rules = {
            "start": [
                {
                    token: "comment",
                    regex: "//.*$"
                },
                {
                    token: "keyword",
                    regex: "\\b(?:" + keywords + ")\\b"
                },
                {
                    token: "variable.parameter",
                    regex: "\\b(?:" + properties + ")\\b"
                },
                {
                    token: "constant.language",
                    regex: "\\b(?:" + colors + ")\\b"
                },
                {
                    token: "constant.language",
                    regex: "\\b(?:" + borderStyles + ")\\b"
                },
                {
                    token: "constant.numeric",
                    regex: "\\b\\d+x\\d+\\b" // Dimensions like 2x3
                },
                {
                    token: "constant.numeric",
                    regex: "-?\\d+\\.\\d+" // Fractional numbers
                },
                {
                    token: "constant.numeric",
                    regex: "\\b\\d+\\b" // Integers
                },
                {
                    token: "string",
                    regex: '"',
                    next: "string"
                },
                {
                    token: "identifier",
                    regex: "[a-zA-Z_][a-zA-Z0-9_]*"
                },
                {
                    token: "paren.lparen",
                    regex: "[\\[({]"
                },
                {
                    token: "paren.rparen",
                    regex: "[\\])}]"
                },
                {
                    token: "punctuation.operator",
                    regex: "[:,.]"
                },
                {
                    token: "text",
                    regex: "\\s+"
                }
            ],
            "string": [
                {
                    token: "string",
                    regex: '"',
                    next: "start"
                },
                {
                    token: "string",
                    regex: '[^"]+'
                }
            ]
        };
    };

    oop.inherits(DiagramyHighlightRules, TextHighlightRules);

    var Mode = function() {
        this.HighlightRules = DiagramyHighlightRules;
    };
    oop.inherits(Mode, TextMode);

    (function() {
        this.$id = "ace/mode/diagramy";
    }).call(Mode.prototype);

    exports.Mode = Mode;
});

