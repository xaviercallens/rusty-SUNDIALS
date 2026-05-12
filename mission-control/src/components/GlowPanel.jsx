export default function GlowPanel({ title, children, className = '', dot = true }) {
  return (
    <div className={`glow-panel animate-in ${className}`}>
      {title && (
        <div className="panel-title">
          {dot && <span className="dot" />}
          {title}
        </div>
      )}
      {children}
    </div>
  );
}
