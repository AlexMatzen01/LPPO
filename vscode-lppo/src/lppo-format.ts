#!/usr/bin/env node

/**
 * LPPO Formatter
 * A simple formatter for the LPPO programming language
 */

interface FormatOptions {
  indentSize: number;
  useTabs: boolean;
  maxLineLength: number;
}

const DEFAULT_OPTIONS: FormatOptions = {
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

function createIndent(level: number, options: FormatOptions): string {
  if (options.useTabs) {
    return '\t'.repeat(level);
  }
  return ' '.repeat(level * options.indentSize);
}

function trimRight(str: string): string {
  return str.replace(/\s+$/, '');
}

function isEmptyOrComment(line: string): boolean {
  const trimmed = line.trim();
  return trimmed === '' || trimmed.startsWith('//') || trimmed.startsWith('/*');
}

function getLineIndent(line: string): number {
  const match = line.match(/^(\s*)/);
  return match ? match[1].length : 0;
}

function countOpenBraces(line: string): number {
  return (line.match(/{/g) || []).length;
}

function countCloseBraces(line: string): number {
  return (line.match(/}/g) || []).length;
}

function countOpenParens(line: string): number {
  return (line.match(/\(/g) || []).length;
}

function countCloseParens(line: string): number {
  return (line.match(/\)/g) || []).length;
}

function countOpenBrackets(line: string): number {
  return (line.match(/\[/g) || []).length;
}

function countCloseBrackets(line: string): number {
  return (line.match(/\]/g) || []).length;
}

function isBlockStart(line: string): boolean {
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

function isBlockEnd(line: string): boolean {
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

function isBlockKeyword(line: string): boolean {
  const trimmed = line.trim();
  for (const kw of BLOCK_KEYWORDS) {
    if (trimmed === kw || trimmed.startsWith(kw + ' ') || trimmed.startsWith(kw + '{')) {
      return true;
    }
  }
  return false;
}

function formatCode(input: string, options: FormatOptions = DEFAULT_OPTIONS): string {
  const lines = input.split('\n');
  const output: string[] = [];
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

function formatWithBraceStyle(input: string, options: FormatOptions = DEFAULT_OPTIONS): string {
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
  let input: string;
  
  if (useStdin) {
    input = await new Promise((resolve) => {
      let data = '';
      process.stdin.on('data', chunk => data += chunk);
      process.stdin.on('end', () => resolve(data));
    });
  } else if (filePath) {
    const fs = await import('fs');
    input = fs.readFileSync(filePath, 'utf-8');
  } else {
    console.error('Usage: lppo-format [--stdin] [file.lppo]');
    process.exit(1);
  }
  
  const options: FormatOptions = {
    indentSize: 4,
    useTabs: false,
    maxLineLength: 100
  };
  
  // Parse options from args
  for (const arg of args) {
    if (arg.startsWith('--indent=')) {
      options.indentSize = parseInt(arg.split('=')[1], 10);
    } else if (arg === '--tabs') {
      options.useTabs = true;
    } else if (arg.startsWith('--max-line-length=')) {
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