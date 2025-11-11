// Simple syntax highlighting for LPPO code
document.addEventListener('DOMContentLoaded', function() {
    // Highlight all code blocks
    document.querySelectorAll('code.language-lppo').forEach(function(block) {
        highlightLPPO(block);
    });
    
    document.querySelectorAll('code.language-bash').forEach(function(block) {
        highlightBash(block);
    });
});

function highlightLPPO(block) {
    let code = block.textContent;
    
    // Keywords
    const keywords = ['proc', 'let', 'out', 'if', 'elif', 'else', 'while', 'for', 'in', 'loop', 
                     'break', 'continue', 'call', 'parallel', 'return', 'and', 'or', 'not'];
    
    // Highlight keywords
    keywords.forEach(keyword => {
        const regex = new RegExp('\\b' + keyword + '\\b', 'g');
        code = code.replace(regex, '<span class="keyword">' + keyword + '</span>');
    });
    
    // Highlight strings
    code = code.replace(/"([^"]*)"/g, '<span class="string">"$1"</span>');
    
    // Highlight numbers
    code = code.replace(/\b(\d+)\b/g, '<span class="number">$1</span>');
    
    // Highlight comments
    code = code.replace(/\/\/(.*?)$/gm, '<span class="comment">//$1</span>');
    code = code.replace(/\/\*([\s\S]*?)\*\//g, '<span class="comment">/*$1*/</span>');
    
    // Highlight function calls
    code = code.replace(/\b([a-zA-Z_][a-zA-Z0-9_]*)\s*\(/g, '<span class="function">$1</span>(');
    
    // Highlight operators
    const operators = ['+', '-', '*', '/', '%', '**', '==', '!=', '<', '>', '<=', '>=', '&', '|', '^', '~'];
    operators.forEach(op => {
        const escaped = op.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        const regex = new RegExp('(' + escaped + ')', 'g');
        code = code.replace(regex, '<span class="operator">$1</span>');
    });
    
    block.innerHTML = code;
}

function highlightBash(block) {
    let code = block.textContent;
    
    // Highlight commands
    code = code.replace(/^(cargo|git|cd|mkdir|ls|cat)(\s|$)/gm, '<span class="keyword">$1</span>$2');
    
    // Highlight strings
    code = code.replace(/"([^"]*)"/g, '<span class="string">"$1"</span>');
    
    // Highlight comments
    code = code.replace(/#(.*?)$/gm, '<span class="comment">#$1</span>');
    
    // Highlight flags
    code = code.replace(/\s(-+[a-zA-Z0-9-]+)/g, ' <span class="operator">$1</span>');
    
    block.innerHTML = code;
}

// Smooth scrolling for anchor links
document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', function (e) {
        e.preventDefault();
        const target = document.querySelector(this.getAttribute('href'));
        if (target) {
            target.scrollIntoView({
                behavior: 'smooth',
                block: 'start'
            });
        }
    });
});

// Active navigation highlighting
window.addEventListener('scroll', function() {
    const sections = document.querySelectorAll('section[id]');
    const navLinks = document.querySelectorAll('.nav-link');
    
    let current = '';
    sections.forEach(section => {
        const sectionTop = section.offsetTop;
        const sectionHeight = section.clientHeight;
        if (pageYOffset >= sectionTop - 200) {
            current = section.getAttribute('id');
        }
    });
    
    navLinks.forEach(link => {
        link.classList.remove('active');
        if (link.getAttribute('href') === '#' + current) {
            link.classList.add('active');
        }
    });
});
