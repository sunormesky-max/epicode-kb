import { useCallback } from 'react'
import { useDropzone } from 'react-dropzone'

const ACCEPTED_TYPES = {
  'text/markdown': ['.md', '.markdown'],
  'text/plain': ['.txt'],
  'application/pdf': ['.pdf'],
}

export default function UploadZone({
  onFile,
  disabled,
}: {
  onFile: (file: File) => void
  disabled?: boolean
}) {
  const onDrop = useCallback(
    (acceptedFiles: File[]) => {
      if (acceptedFiles.length > 0) {
        onFile(acceptedFiles[0])
      }
    },
    [onFile],
  )

  const { getRootProps, getInputProps, isDragActive, isDragReject } = useDropzone({
    onDrop,
    accept: ACCEPTED_TYPES,
    maxFiles: 1,
    disabled,
  })

  return (
    <div
      {...getRootProps()}
      className={`border-2 border-dashed rounded-xl p-12 text-center cursor-pointer transition-colors ${
        disabled
          ? 'border-gray-300 bg-gray-50 opacity-60 cursor-not-allowed'
          : isDragActive
            ? 'border-blue-500 bg-blue-50'
            : isDragReject
              ? 'border-red-500 bg-red-50'
              : 'border-gray-300 hover:border-blue-400 hover:bg-gray-50'
      }`}
    >
      <input {...getInputProps()} />
      <div className="text-4xl mb-3">📄</div>
      {disabled ? (
        <p className="text-gray-500">Uploading...</p>
      ) : isDragActive ? (
        <p className="text-blue-600 font-medium">Drop the file here...</p>
      ) : (
        <>
          <p className="text-gray-700 font-medium mb-1">
            Drag & drop a file here, or click to select
          </p>
          <p className="text-sm text-gray-500">
            Supports: .md, .txt, .pdf (max 10MB)
          </p>
        </>
      )}
    </div>
  )
}
