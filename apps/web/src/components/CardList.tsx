import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { GripVertical } from 'lucide-react';
import type { CardOptions } from '@gitcard/svg-renderer';
import type { CardType } from '@/hooks/useCardConfig';
import type { GitHubData } from '@/hooks/useGitHubData';
import { CardPreview } from './CardPreview';

interface CardListProps {
  cards: CardType[];
  onReorder: (cards: CardType[]) => void;
  data: GitHubData;
  options: CardOptions;
}

interface SortableCardProps {
  id: CardType;
  data: GitHubData;
  options: CardOptions;
}

function SortableCard({ id, data, options }: SortableCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({ id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className="rounded-lg border bg-card shadow-sm cursor-grab overflow-hidden transition-shadow hover:shadow-md active:cursor-grabbing active:shadow-lg"
      {...attributes}
      {...listeners}
    >
      <div className="flex items-center gap-2 px-3 py-2 border-b bg-muted/50">
        <GripVertical className="h-4 w-4 text-muted-foreground" />
      </div>
      <CardPreview id={id} data={data} options={options} />
    </div>
  );
}

export function CardList({ cards, onReorder, data, options }: CardListProps) {
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;

    if (over && active.id !== over.id) {
      const oldIndex = cards.indexOf(active.id as CardType);
      const newIndex = cards.indexOf(over.id as CardType);
      onReorder(arrayMove(cards, oldIndex, newIndex));
    }
  }

  return (
    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
      <SortableContext items={cards} strategy={verticalListSortingStrategy}>
        <div className="flex flex-col gap-4">
          {cards.map((card) => (
            <SortableCard key={card} id={card} data={data} options={options} />
          ))}
        </div>
      </SortableContext>
    </DndContext>
  );
}
