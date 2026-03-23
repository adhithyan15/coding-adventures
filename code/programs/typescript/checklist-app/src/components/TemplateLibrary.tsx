/**
 * TemplateLibrary — Screen 1: the home screen listing all templates.
 *
 * Renders a grid of TemplateCards. Each card has Run / Edit / Delete actions.
 * Run creates a new instance and navigates to the Instance Runner.
 * Edit navigates to the Template Editor in edit mode.
 * Delete removes the template after a window.confirm guard.
 *
 * The "New Checklist" button navigates to the Template Editor in create mode.
 */

import { useTranslation } from "@coding-adventures/ui-components";
import { appState, createInstance, deleteTemplate } from "../state.js";
import type { Template } from "../types.js";

interface TemplateLibraryProps {
  onNavigate: (path: string) => void;
}

function countItems(items: Template["items"]): number {
  let n = 0;
  for (const item of items) {
    n++;
    if (item.type === "decision") {
      n += countItems(item.yesBranch);
      n += countItems(item.noBranch);
    }
  }
  return n;
}

interface TemplateCardProps {
  template: Template;
  onRun: () => void;
  onEdit: () => void;
  onDelete: () => void;
}

function TemplateCard({ template, onRun, onEdit, onDelete }: TemplateCardProps) {
  const { t } = useTranslation();
  const total = countItems(template.items);

  return (
    <article className="template-card" aria-label={template.name}>
      <h2 className="template-card__name">{template.name}</h2>
      {template.description && (
        <p className="template-card__description">{template.description}</p>
      )}
      <p className="template-card__meta">
        {total === 1
          ? t("library.itemCount.singular")
          : t("library.itemCount").replace("{count}", String(total))}
      </p>
      <div className="template-card__actions">
        <button className="btn--primary" onClick={onRun}>
          {t("library.run")}
        </button>
        <button className="btn--secondary" onClick={onEdit}>
          {t("library.edit")}
        </button>
        <button className="btn--danger" onClick={onDelete}>
          {t("library.delete")}
        </button>
      </div>
    </article>
  );
}

export function TemplateLibrary({ onNavigate }: TemplateLibraryProps) {
  const { t } = useTranslation();
  const templates = appState.templates;

  function handleRun(templateId: string) {
    const instance = createInstance(appState, templateId);
    onNavigate(`/instance/${instance.id}`);
  }

  function handleDelete(template: Template) {
    const confirmed = window.confirm(
      t("library.deleteConfirm").replace("{name}", template.name),
    );
    if (confirmed) {
      deleteTemplate(appState, template.id);
      onNavigate("/");
    }
  }

  return (
    <section aria-label={t("library.title")}>
      <div className="template-library__actions">
        <button
          className="btn--primary"
          onClick={() => onNavigate("/template/new")}
        >
          {t("library.newTemplate")}
        </button>
      </div>

      {templates.length === 0 ? (
        <p className="template-library__empty">{t("library.empty")}</p>
      ) : (
        <div className="template-library__grid">
          {templates.map((template) => (
            <TemplateCard
              key={template.id}
              template={template}
              onRun={() => handleRun(template.id)}
              onEdit={() => onNavigate(`/template/${template.id}/edit`)}
              onDelete={() => handleDelete(template)}
            />
          ))}
        </div>
      )}
    </section>
  );
}
