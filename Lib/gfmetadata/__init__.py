# Re-export all the things for convenience
from gfmetadata.axes_pb2 import AxisProto, FallbackProto
from gfmetadata.designers_pb2 import DesignerInfoProto, AvatarProto
from gfmetadata.fonts_public_pb2 import (
    AxisSegmentProto,
    FamilyFallbackProto,
    FamilyProto,
    FontProto,
    GlyphGroupProto,
    SampleTextProto as FamilySampleTextProto,
    SourceFileProto,
    SourceProto,
    TargetProto,
    TargetTypeProto,
)
from gfmetadata.knowledge_pb2 import (
    ContributorsProto,
    KnowledgeProto,
    LessonProto,
    ModuleProto,
    TermProto,
    TopicProto,
)
from gfmetadata.languages_public_pb2 import (
    ExemplarCharsProto,
    LanguageProto,
    RegionProto,
    SampleTextProto,
    ScriptProto,
)
# Re-export so users get the same protobuf version
from google.protobuf import text_format

__all__ = [
    "AxisProto",
    "FallbackProto",
    "DesignerInfoProto",
    "AvatarProto",
    "AxisSegmentProto",
    "FamilyFallbackProto",
    "FamilyProto",
    "FontProto",
    "GlyphGroupProto",
    "FamilySampleTextProto",
    "SourceFileProto",
    "SourceProto",
    "TargetProto",
    "TargetTypeProto",
    "ContributorsProto",
    "KnowledgeProto",
    "LessonProto",
    "ModuleProto",
    "TermProto",
    "TopicProto",
    "ExemplarCharsProto",
    "LanguageProto",
    "RegionProto",
    "SampleTextProto",
    "ScriptProto",
    "text_format",
]