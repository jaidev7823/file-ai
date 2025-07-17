import React from 'react';
import { FileText, File, Image, Music, Video, Folder } from 'lucide-react';

const getFileIcon = (fileType) => {
  switch (fileType) {
    case 'text':
    case 'txt':
    case 'md':
      return FileText;
    case 'pdf':
    case 'doc':
    case 'docx':
      return FileText;
    case 'image':
    case 'jpg':
    case 'png':
    case 'gif':
      return Image;
    case 'audio':
    case 'mp3':
    case 'wav':
      return Music;
    case 'video':
    case 'mp4':
    case 'avi':
      return Video;
    case 'folder':
      return Folder;
    default:
      return File;
  }
};

const ResultItem = ({ file, onClick, isSelected }) => {
  const IconComponent = getFileIcon(file.type);
  
  return (
    <div
      onClick={() => onClick(file)}
      className={`flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-all duration-150 ${
        isSelected 
          ? 'bg-blue-50 border-l-4 border-blue-500' 
          : 'hover:bg-gray-50'
      }`}
    >
      <IconComponent className="h-5 w-5 text-gray-500 flex-shrink-0" />
      <div className="flex-1 min-w-0">
        <div className="font-medium text-gray-900 truncate">
          {file.name}
        </div>
        <div className="text-sm text-gray-500 truncate">
          {file.path}
        </div>
        {file.snippet && (
          <div className="text-xs text-gray-400 mt-1 line-clamp-2">
            {file.snippet}
          </div>
        )}
      </div>
      <div className="text-xs text-gray-400">
        {file.score && `${Math.round(file.score * 100)}%`}
      </div>
    </div>
  );
};

const ResultsList = ({ results, onFileOpen, selectedIndex }) => {
  if (!results || results.length === 0) {
    return (
      <div className="text-center py-8 text-gray-500">
        <File className="h-12 w-12 mx-auto mb-3 text-gray-300" />
        <p>No files found</p>
        <p className="text-sm mt-1">Try a different search term</p>
      </div>
    );
  }

  return (
    <div className="mt-4 max-h-96 overflow-y-auto">
      <div className="space-y-1">
        {results.map((file, index) => (
          <ResultItem
            key={`${file.path}-${index}`}
            file={file}
            onClick={onFileOpen}
            isSelected={index === selectedIndex}
          />
        ))}
      </div>
    </div>
  );
};

export default ResultsList;