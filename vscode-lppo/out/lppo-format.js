#!/usr/bin/env node
"use strict";
/**
 * LPPO Formatter
 * A simple formatter for the LPPO programming language
 */
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
const DEFAULT_OPTIONS = {
    indentSize: 4,
    useTabs: false,
    maxLineLength: 100
};
// Keywords that increase indentation
const BLOCK_START_KEYWORDS = [
    'proc', 'parallel', 'if', 'elif', 'else', 'while', 'for', 'loop',
    'match', 'struct', 'enum', 'trait', 'impl', 'module', 'scope',
    'beginseg', 'endseg', 'asyncblk', 'catchblk', 'finallyblk',
    'try', 'catch', 'finally', 'select', 'case', 'default',
    'macro', 'macro_rules', 'macro_use', 'macro_export',
    'unsafe', 'defer', 'generator', 'coroutine', 'async',
    'do', 'then', 'when', 'where', 'with', 'without',
    'repeat', 'until', 'unless'
];
// Keywords that decrease indentation
const BLOCK_END_KEYWORDS = [
    'end', 'endseg', 'endfor', 'endwhile', 'endif', 'endloop',
    'endmatch', 'endstruct', 'endenum', 'endtrait', 'endimpl',
    'endmodule', 'endscope', 'endasyncblk', 'endcatchblk',
    'endfinallyblk', 'endtry', 'endcatch', 'endfinally',
    'endselect', 'endcase', 'enddefault', 'endmacro',
    'endmacro_rules', 'endmacro_use', 'endmacro_export',
    'endunsafe', 'enddefer', 'endgenerator', 'endcoroutine',
    'endasync', 'enddo', 'endthen', 'endwhen', 'endwhere',
    'endwith', 'endwithout', 'endrepeat', 'enduntil', 'endunless'
];
// Keywords that don't change indentation but are block-related
const BLOCK_KEYWORDS = [
    'else', 'elif', 'catch', 'finally', 'case', 'default',
    'fallthrough'
];
function createIndent(level, options) {
    if (options.useTabs) {
        return '\t'.repeat(level);
    }
    return ' '.repeat(level * options.indentSize);
}
function trimRight(str) {
    return str.replace(/\s+$/, '');
}
function isEmptyOrComment(line) {
    const trimmed = line.trim();
    return trimmed === '' || trimmed.startsWith('//') || trimmed.startsWith('/*');
}
function getLineIndent(line) {
    const match = line.match(/^(\s*)/);
    return match ? match[1].length : 0;
}
function countOpenBraces(line) {
    return (line.match(/{/g) || []).length;
}
function countCloseBraces(line) {
    return (line.match(/}/g) || []).length;
}
function countOpenParens(line) {
    return (line.match(/\(/g) || []).length;
}
function countCloseParens(line) {
    return (line.match(/\)/g) || []).length;
}
function countOpenBrackets(line) {
    return (line.match(/\[/g) || []).length;
}
function countCloseBrackets(line) {
    return (line.match(/\]/g) || []).length;
}
function isBlockStart(line) {
    const trimmed = line.trim();
    // Check for block start keywords followed by {
    for (const kw of BLOCK_START_KEYWORDS) {
        const regex = new RegExp(`^${kw}\\b.*\\{$`);
        if (regex.test(trimmed)) {
            return true;
        }
    }
    return false;
}
function isBlockEnd(line) {
    const trimmed = line.trim();
    // Check for closing brace or end keywords
    if (trimmed === '}' || trimmed.startsWith('} ')) {
        return true;
    }
    for (const kw of BLOCK_END_KEYWORDS) {
        if (trimmed === kw || trimmed.startsWith(kw + ' ')) {
            return true;
        }
    }
    return false;
}
function isBlockKeyword(line) {
    const trimmed = line.trim();
    for (const kw of BLOCK_KEYWORDS) {
        if (trimmed === kw || trimmed.startsWith(kw + ' ') || trimmed.startsWith(kw + '{')) {
            return true;
        }
    }
    return false;
}
function formatCode(input, options = DEFAULT_OPTIONS) {
    const lines = input.split('\n');
    const output = [];
    let indentLevel = 0;
    let braceStack = 0;
    let parenStack = 0;
    let bracketStack = 0;
    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const trimmed = line.trim();
        // Skip empty lines but preserve them
        if (isEmptyOrComment(line)) {
            output.push(line);
            continue;
        }
        // Calculate current indentation based on brace/paren/bracket counts
        const openBraces = countOpenBraces(line);
        const closeBraces = countCloseBraces(line);
        const openParens = countOpenParens(line);
        const closeParens = countCloseParens(line);
        const openBrackets = countOpenBrackets(line);
        const closeBrackets = countCloseBrackets(line);
        // Adjust indent for closing braces on this line
        if (closeBraces > 0) {
            indentLevel = Math.max(0, indentLevel - closeBraces);
        }
        // Adjust for block keywords (else, elif, catch, etc.)
        if (isBlockKeyword(trimmed)) {
            indentLevel = Math.max(0, indentLevel - 1);
        }
        // Apply indentation
        const indent = createIndent(indentLevel, options);
        output.push(indent + trimmed);
        // Update stacks
        braceStack += openBraces - closeBraces;
        parenStack += openParens - closeParens;
        bracketStack += openBrackets - closeBrackets;
        // Adjust indent for opening braces on this line
        if (openBraces > 0) {
            indentLevel += openBraces;
        }
        // Check for block start keywords
        if (isBlockStart(trimmed)) {
            // Already handled by open braces
        }
        // Handle case where line ends with { but we didn't count it
        if (trimmed.endsWith('{') && !trimmed.endsWith('{}')) {
            // Already counted in openBraces
        }
    }
    // Post-process: clean up extra blank lines
    const result = output.join('\n');
    // Remove trailing whitespace from each line
    return result.split('\n').map(trimRight).join('\n');
}
function formatWithBraceStyle(input, options = DEFAULT_OPTIONS) {
    // First pass: basic formatting
    let result = formatCode(input, options);
    // Second pass: ensure consistent brace style (K&R style)
    // { on same line as declaration
    result = result.replace(/\)\s*\n\s*{/g, ') {');
    result = result.replace(/}\s*\n\s*{/g, '} {');
    result = result.replace(/:\s*\n\s*{/g, ': {');
    // Ensure space after keywords
    result = result.replace(/\b(if|while|for|elif|else|catch|finally|match|select|case|default)\s*\(/g, '$1 (');
    // Ensure space around operators
    result = result.replace(/(\w)([=+\-*/%<>&|^!]=?)(\w)/g, '$1 $2 $3');
    result = result.replace(/([=+\-*/%<>&|^!]=?)\s+/g, '$1 ');
    result = result.replace(/\s+([=+\-*/%<>&|^!]=?)/g, ' $1');
    // Fix multiple spaces
    result = result.replace(/  +/g, ' ');
    // Ensure space after commas
    result = result.replace(/,(?!\s)/g, ', ');
    // Ensure space after semicolons in for loops
    result = result.replace(/;(?!\s)/g, '; ');
    return result;
}
// Read from stdin if no file provided
const args = process.argv.slice(2);
const useStdin = args.includes('--stdin');
const filePath = args.find(arg => !arg.startsWith('--'));
async function main() {
    let input;
    if (useStdin) {
        input = await new Promise((resolve) => {
            let data = '';
            process.stdin.on('data', chunk => data += chunk);
            process.stdin.on('end', () => resolve(data));
        });
    }
    else if (filePath) {
        const fs = await Promise.resolve().then(() => __importStar(require('fs')));
        input = fs.readFileSync(filePath, 'utf-8');
    }
    else {
        console.error('Usage: lppo-format [--stdin] [file.lppo]');
        process.exit(1);
    }
    const options = {
        indentSize: 4,
        useTabs: false,
        maxLineLength: 100
    };
    // Parse options from args
    for (const arg of args) {
        if (arg.startsWith('--indent=')) {
            options.indentSize = parseInt(arg.split('=')[1], 10);
        }
        else if (arg === '--tabs') {
            options.useTabs = true;
        }
        else if (arg.startsWith('--max-line-length=')) {
            options.maxLineLength = parseInt(arg.split('=')[1], 10);
        }
    }
    const formatted = formatWithBraceStyle(input, options);
    console.log(formatted);
}
main().catch(err => {
    console.error('Error:', err);
    process.exit(1);
});
//# sourceMappingURL=lppo-format.js.map