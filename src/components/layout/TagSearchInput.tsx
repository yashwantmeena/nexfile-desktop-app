import { useEffect, useId, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Props {
  query: string;
  enabled: boolean;
  placeholder: string;
  onChange: (value: string) => void;
  onSubmit: (value: string) => void;
}

export function TagSearchInput({ query, enabled, placeholder, onChange, onSubmit }: Props) {
  const listId = useId();
  const [focused, setFocused] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  const [result, setResult] = useState<{ query: string; values: string[] }>({ query: "", values: [] });
  const [active, setActive] = useState(-1);
  const composing = useRef(false);
  const values = enabled && focused && !dismissed && result.query === query ? result.values : [];

  useEffect(() => {
    let cancelled = false;
    setActive(-1);
    setResult({ query, values: [] });
    if (!enabled || !focused || dismissed || [...query.trim()].length < 2 || query.length > 256) return;
    const timer = window.setTimeout(() => {
      void invoke<string[]>("suggest_tags", { prefix: query }).then(values => {
        if (!cancelled) setResult({ query, values });
      }).catch(() => {
        // Suggestions are optional; typing remains available if the index cannot be read.
        if (!cancelled) setResult({ query, values: [] });
      });
    }, 150);
    return () => { cancelled = true; window.clearTimeout(timer); };
  }, [query, enabled, focused, dismissed]);

  const select = (value: string) => {
    setDismissed(true);
    setActive(-1);
    onChange(value);
    onSubmit(value);
  };

  return <div className="tag-search-input">
    <input aria-label="Search files" role="combobox" aria-autocomplete="list"
      aria-expanded={values.length > 0} aria-controls={values.length ? listId : undefined}
      aria-activedescendant={active >= 0 && active < values.length ? `${listId}-${active}` : undefined}
      autoComplete="off" value={query} placeholder={placeholder}
      onFocus={() => { setFocused(true); setDismissed(false); }}
      onBlur={() => { setFocused(false); setDismissed(true); }}
      onCompositionStart={() => { composing.current = true; setDismissed(true); }}
      onCompositionEnd={() => { composing.current = false; setDismissed(false); }}
      onChange={event => { setDismissed(composing.current); setActive(-1); onChange(event.target.value); }}
      onKeyDown={event => {
        if (event.nativeEvent.isComposing) return;
        if (event.key === "Escape") { setDismissed(true); setActive(-1); }
        if (event.key === "Enter") {
          event.preventDefault();
          select(active >= 0 && active < values.length ? values[active] : query);
          return;
        }
        if (!values.length) return;
        if (event.key === "ArrowDown" || event.key === "ArrowUp") {
          event.preventDefault();
          setActive(previous => event.key === "ArrowDown"
            ? (previous + 1) % values.length
            : (previous <= 0 ? values.length - 1 : previous - 1));
        }
      }} />
    {values.length > 0 && <div className="tag-suggestions" id={listId} role="listbox" aria-label="Tag suggestions">
      {values.map((value, index) => <div key={value} id={`${listId}-${index}`} role="option"
        aria-selected={active === index} className={active === index ? "active" : ""}
        onPointerDown={event => event.preventDefault()} onMouseMove={() => setActive(index)}
        onClick={() => select(value)}>{value}</div>)}
    </div>}
  </div>;
}
