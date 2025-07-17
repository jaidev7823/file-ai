import React, { useState, useEffect, useRef } from 'react';
import { Search } from 'lucide-react';

const SearchBox = ({ onSearch, onClose }) => {
  const [query, setQuery] = useState('');
  const inputRef = useRef(null);

  useEffect(() => {
    // Auto-focus when component mounts
    if (inputRef.current) {
      inputRef.current.focus();
    }
  }, []);

  const handleSubmit = (e) => {
    e.preventDefault();
    if (query.trim()) {
      onSearch(query.trim());
    }
  };

  const handleKeyDown = (e) => {
    if (e.key === 'Escape') {
      onClose();
    }
  };

  return (
    <div className="w-full max-w-2xl mx-auto">
      <form onSubmit={handleSubmit} className="relative">
        <div className="relative flex items-center">
          <Search className="absolute left-4 h-5 w-5 text-gray-400" />
          <input
            ref={inputRef}
            type="text"
            placeholder="Search your files..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            className="w-full pl-12 pr-4 py-4 text-lg bg-white/90 backdrop-blur-sm border border-gray-200 rounded-2xl shadow-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all duration-200"
          />
        </div>
        {query && (
          <div className="absolute right-4 top-1/2 transform -translate-y-1/2">
            <kbd className="px-2 py-1 text-xs bg-gray-100 rounded border text-gray-500">
              Enter
            </kbd>
          </div>
        )}
      </form>
    </div>
  );
};

export default SearchBox;