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
declare const DEFAULT_OPTIONS: FormatOptions;
declare const BLOCK_START_KEYWORDS: string[];
declare const BLOCK_END_KEYWORDS: string[];
declare const BLOCK_KEYWORDS: string[];
declare function createIndent(level: number, options: FormatOptions): string;
declare function trimRight(str: string): string;
declare function isEmptyOrComment(line: string): boolean;
declare function getLineIndent(line: string): number;
declare function countOpenBraces(line: string): number;
declare function countCloseBraces(line: string): number;
declare function countOpenParens(line: string): number;
declare function countCloseParens(line: string): number;
declare function countOpenBrackets(line: string): number;
declare function countCloseBrackets(line: string): number;
declare function isBlockStart(line: string): boolean;
declare function isBlockEnd(line: string): boolean;
declare function isBlockKeyword(line: string): boolean;
declare function formatCode(input: string, options?: FormatOptions): string;
declare function formatWithBraceStyle(input: string, options?: FormatOptions): string;
declare const args: string[];
declare const useStdin: boolean;
declare const filePath: string | undefined;
declare function main(): Promise<void>;
//# sourceMappingURL=lppo-format.d.ts.map