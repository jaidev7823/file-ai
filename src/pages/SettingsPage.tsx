import React, { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";

// A generic component to manage a list of rules
interface RuleListProps {
  title: string;
  description: string;
  placeholder: string;
  rules: string[];
  onAdd: (rule: string) => void;
  onRemove: (rule: string) => void;
  inputType?: string;
}

const RuleList: React.FC<RuleListProps> = ({
  title,
  description,
  placeholder,
  rules,
  onAdd,
  onRemove,
  inputType = "text",
}) => {
  const [inputValue, setInputValue] = useState("");

  const handleAddClick = () => {
    if (inputValue.trim()) {
      onAdd(inputValue.trim());
      setInputValue("");
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      handleAddClick();
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex w-full max-w-sm items-center space-x-2">
          <Input
            type={inputType}
            placeholder={placeholder}
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyPress={handleKeyPress}
          />
          <Button onClick={handleAddClick}>Add</Button>
        </div>
        <Separator className="my-4" />
        <div className="flex flex-wrap gap-2">
          {rules.length > 0 ? (
            rules.map((rule, index) => (
              <Badge
                key={index}
                variant="secondary"
                className="flex items-center gap-2 text-sm"
              >
                <span>{rule}</span>
                <button
                  onClick={() => onRemove(rule)}
                  className="rounded-full hover:bg-muted-foreground/20 p-0.5"
                  aria-label={`Remove ${rule}`}
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                  </svg>
                </button>
              </Badge>
            ))
          ) : (
            <p className="text-sm text-muted-foreground">No rules added.</p>
          )}
        </div>
      </CardContent>
    </Card>
  );
};

export default function SettingsPage() {
  // These states are for UI demonstration purposes.
  // You will replace them with data fetched from your backend.
  const [includedPaths, setIncludedPaths] = useState<string[]>([
    "C:\\Users\\Jai Mishra\\Documents",
  ]);
  const [excludedPaths, setExcludedPaths] = useState<string[]>([
    "C:\\Users\\Jai Mishra\\Documents\\node_modules",
  ]);
  const [includedExtensions, setIncludedExtensions] = useState<string[]>([
    ".txt",
    ".md",
    ".pdf",
    ".docx",
  ]);
    const [excludedFilenames, setExcludedFilenames] = useState<string[]>([
    "~$temp.docx",
  ]);


  return (
    <div className="p-6 space-y-6 max-w-4xl mx-auto">
      <div>
        <h1 className="text-3xl font-bold">Scan Settings</h1>
        <p className="text-muted-foreground mt-1">
          Manage the rules for file and folder scanning. Changes will be applied
          on the next scan.
        </p>
      </div>
      <Separator />

      <div className="space-y-4">
        <RuleList
          title="Folders to Scan"
          description="Add the absolute paths of folders you want to include in the scan."
          placeholder="e.g., C:\Users\YourName\Projects"
          rules={includedPaths}
          onAdd={(rule) => setIncludedPaths([...includedPaths, rule])}
          onRemove={(rule) =>
            setIncludedPaths(includedPaths.filter((r) => r !== rule))
          }
        />
        <RuleList
          title="Folders to Ignore"
          description="Add the absolute paths of folders to exclude from the scan."
          placeholder="e.g., C:\Users\YourName\Projects\node_modules"
          rules={excludedPaths}
          onAdd={(rule) => setExcludedPaths([...excludedPaths, rule])}
          onRemove={(rule) =>
            setExcludedPaths(excludedPaths.filter((r) => r !== rule))
          }
        />
        <RuleList
          title="File Extensions to Scan"
          description="Specify which file types to include (e.g., .txt, .md)."
          placeholder="e.g., .html"
          rules={includedExtensions}
          onAdd={(rule) => setIncludedExtensions([...includedExtensions, rule])}
          onRemove={(rule) =>
            setIncludedExtensions(includedExtensions.filter((r) => r !== rule))
          }
        />
        <RuleList
          title="Filenames to Ignore"
          description="Specify exact filenames to exclude from the scan."
          placeholder="e.g., temp.log"
          rules={excludedFilenames}
          onAdd={(rule) => setExcludedFilenames([...excludedFilenames, rule])}
          onRemove={(rule) =>
            setExcludedFilenames(excludedFilenames.filter((r) => r !== rule))
          }
        />
      </div>
    </div>
  );
}
